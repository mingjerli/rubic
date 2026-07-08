//! Bevy wiring for camera cube input (spec 0002, Phase B).
//!
//! **Compile-verified only.** This drives the tested vision pipeline
//! ([`crate::vision`]) from a live [`CameraSource`], but a real camera and
//! display are needed to exercise it, so behavior here is validated on-device,
//! not in tests. The one piece of pure logic — handing a completed scan to the
//! paint-review state — is unit-tested below.
//!
//! A live video preview streams each camera frame into a fixed-size texture
//! ([`setup_camera_preview`] / [`upload_preview`]) shown while scanning, plus a
//! text HUD for the target face and progress.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::{RgbImage, imageops};
use rubic_core::{Face, PartialFacelets};

use crate::mode::AppMode;
use crate::paint::InputState;
use crate::colors::sticker_rgb;
use crate::vision::Rgb;
use crate::vision::capture::{CaptureEvent, CaptureFlow};
use crate::vision::classify::Classified;
use crate::vision::color::{perceptual_point, point_distance_sq};
use crate::vision::pipeline::{capture_centered, read_face_grid, read_face_grid_detail};
use crate::vision::source::CameraSource;

/// A read face for the preview overlay: nine colors + their fitted centers.
type FaceRead = ([Rgb; 9], [(f32, f32); 9]);

/// Fixed preview texture size; incoming frames are resized to this, so the
/// texture never needs reallocating.
const PREVIEW_W: u32 = 480;
const PREVIEW_H: u32 = 360;

/// On-screen preview size: a small 4:3 inset tucked in the bottom-right corner,
/// clear of the centered 3D cube and the bottom-left Mode/Status HUD.
/// (Independent of the texture resolution.)
const DISPLAY_W: f32 = 360.0;
const DISPLAY_H: f32 = 270.0;
/// Margin of the preview/HUD from the window's bottom-right corner.
const CORNER_MARGIN: f32 = 10.0;

/// Handle to the live-preview texture that camera frames are streamed into.
#[derive(Resource)]
pub struct PreviewImage(pub Handle<Image>);

/// Marker for the on-screen preview UI node.
#[derive(Component)]
pub struct PreviewNode;

/// The in-progress camera scan.
#[derive(Resource, Default)]
pub struct CameraSession {
    /// The guided capture state machine.
    pub flow: CaptureFlow,
    /// The most recent capture event, for the HUD.
    pub last_event: CaptureEvent,
    /// Whether the latest processed frame produced a readable face (for the HUD
    /// "ready to capture" hint).
    pub detected: bool,
}

/// The live camera, if one was opened. Held as a non-send resource because a
/// native camera handle is not `Sync`.
pub struct CameraFeed(pub Option<Box<dyn CameraSource>>);

/// Marker for the camera-scan HUD text.
#[derive(Component)]
pub struct CameraHud;

/// Convert a completed scan into paint-review state (the hand-off point).
#[must_use]
pub fn handoff(classified: &Classified) -> PartialFacelets {
    PartialFacelets::from_facelets(&classified.facelets)
}

/// Startup: create the preview texture and spawn the (hidden) preview UI node.
pub fn setup_camera_preview(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = Image::new(
        Extent3d {
            width: PREVIEW_W,
            height: PREVIEW_H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // Opaque gray so the box is visible before any camera frame arrives.
        [70u8, 70, 85, 255].repeat((PREVIEW_W * PREVIEW_H) as usize),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);
    // Alignment box, sized to match the sampled guide region (3/5 of the shorter
    // side). Preview is 4:3, so that is 45% of the width and 60% of the height.
    let box_w = 100.0 * 3.0 / 5.0 * PREVIEW_H as f32 / PREVIEW_W as f32; // width %
    let red = Color::srgb(1.0, 0.2, 0.2);
    commands
        .spawn((
            ImageNode::new(handle.clone()),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(CORNER_MARGIN),
                right: Val::Px(CORNER_MARGIN),
                width: Val::Px(DISPLAY_W),
                height: Val::Px(DISPLAY_H),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor(Color::srgb(0.4, 0.7, 1.0)),
            Visibility::Visible,
            PreviewNode,
        ))
        .with_children(|parent| {
            // The 3x3 alignment grid: a bordered box plus inner grid lines.
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent((100.0 - box_w) / 2.0),
                        top: Val::Percent(20.0),
                        width: Val::Percent(box_w),
                        height: Val::Percent(60.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(red),
                ))
                .with_children(|grid| {
                    for third in [100.0 / 3.0, 200.0 / 3.0] {
                        // Vertical line.
                        grid.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(third),
                                top: Val::Percent(0.0),
                                width: Val::Px(2.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(red),
                        ));
                        // Horizontal line.
                        grid.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Percent(third),
                                left: Val::Percent(0.0),
                                height: Val::Px(2.0),
                                width: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(red),
                        ));
                    }
                });
        });
    commands.insert_resource(PreviewImage(handle));
}

/// Resize `frame` to the preview texture, mark each read cell with the color
/// sampled there (real-time labeling), and upload it as RGBA.
fn upload_preview(
    frame: &RgbImage,
    images: &mut Assets<Image>,
    handle: &Handle<Image>,
    overlay: Option<&FaceRead>,
) {
    let mut resized = imageops::resize(frame, PREVIEW_W, PREVIEW_H, imageops::FilterType::Triangle);
    if let Some((colors, centers)) = overlay {
        let (w, h) = frame.dimensions();
        let sx = PREVIEW_W as f32 / w as f32;
        let sy = PREVIEW_H as f32 / h as f32;
        for (color, &(cx, cy)) in colors.iter().zip(centers.iter()) {
            let p = (cx * sx, cy * sy);
            // White ring + the color read at that cell.
            imageproc::drawing::draw_filled_circle_mut(
                &mut resized,
                (p.0 as i32, p.1 as i32),
                6,
                image::Rgb([255, 255, 255]),
            );
            imageproc::drawing::draw_filled_circle_mut(
                &mut resized,
                (p.0 as i32, p.1 as i32),
                4,
                image::Rgb(*color),
            );
        }
    }
    if let Some(image) = images.get_mut(handle) {
        let rgba: Vec<u8> = resized
            .pixels()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect();
        image.data = Some(rgba);
    }
}

/// Show the live preview during input and scanning; hide it while solving.
pub fn toggle_preview(mode: Res<AppMode>, mut nodes: Query<&mut Visibility, With<PreviewNode>>) {
    let want = if *mode == AppMode::Solve {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    for mut vis in &mut nodes {
        if *vis != want {
            *vis = want;
        }
    }
}

/// Startup: spawn the camera-scan HUD as a banner just above the bottom-right
/// preview, hidden until scanning so it never overlaps the app's other UI.
pub fn setup_camera_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.97, 1.0)),
        TextLayout::new_with_justify(JustifyText::Center),
        Node {
            position_type: PositionType::Absolute,
            // Directly above the preview, same right edge and width.
            bottom: Val::Px(CORNER_MARGIN + DISPLAY_H + 6.0),
            right: Val::Px(CORNER_MARGIN),
            width: Val::Px(DISPLAY_W),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.85)),
        Visibility::Hidden,
        CameraHud,
    ));
}

/// In Input mode, `C` enters camera-scan mode (only if a camera was opened).
pub fn enter_camera_scan(
    keys: Res<ButtonInput<KeyCode>>,
    feed: NonSend<CameraFeed>,
    mut mode: ResMut<AppMode>,
    mut session: ResMut<CameraSession>,
) {
    if keys.just_pressed(KeyCode::KeyC) && feed.0.is_some() {
        session.flow.reset();
        *mode = AppMode::Camera;
    }
}

/// In camera mode, `Escape`/`Tab` returns to Input; `R` restarts the scan from
/// the first face; `Enter` (or `Space`) captures the current target face from
/// the latest frame, like snapping a photo in a check-scanner.
pub fn camera_scan_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut feed: NonSendMut<CameraFeed>,
    mut session: ResMut<CameraSession>,
    mut mode: ResMut<AppMode>,
    mut input: ResMut<InputState>,
) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Tab) {
        *mode = AppMode::Input;
        return;
    }
    // Restart the scan, discarding every captured face (net back to centers).
    if keys.just_pressed(KeyCode::KeyR) {
        session.flow.reset();
        session.last_event = CaptureEvent::Idle;
        input.partial = PartialFacelets::new();
        return;
    }
    let capture = keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || keys.just_pressed(KeyCode::Space);
    if capture {
        if let Some(src) = feed.0.as_mut() {
            if let Some(frame) = src.next_frame() {
                // Use the detected face; fall back to the centered grid so a
                // capture always succeeds even if detection missed this frame.
                let samples = read_face_grid(&frame).unwrap_or_else(|| capture_centered(&frame));
                let target = session.flow.current_target();
                let event = session.flow.force_capture(samples);
                session.last_event = event;
                // Live net fill: paint the just-captured face onto the 2D net
                // right away (approximate scheme colors) so a bad read is
                // visible immediately; the final classify refines it at the end.
                if let Some(face) = target {
                    for (k, &s) in samples.iter().enumerate() {
                        input.partial = input.partial.set(face.index() * 9 + k, nearest_scheme_face(s));
                    }
                }
                finish_if_complete(event, &mut session, &mut mode, &mut input);
            }
        }
    }
}

/// Every tick, pull a frame into the live preview. On a detection cadence, run
/// face detection so the preview shows the read colors and the HUD knows
/// whether a face is ready to capture. Capture itself is manual (see
/// [`camera_scan_controls`]) — like lining a check up before snapping it.
pub fn pump_camera(
    mut feed: NonSendMut<CameraFeed>,
    mut session: ResMut<CameraSession>,
    preview: Res<PreviewImage>,
    mut images: ResMut<Assets<Image>>,
    mut frame_count: Local<u64>,
    mut warned_empty: Local<bool>,
    mut last_read: Local<Option<FaceRead>>,
) {
    let Some(src) = feed.0.as_mut() else {
        return;
    };
    let Some(frame) = src.next_frame() else {
        if !*warned_empty {
            eprintln!("rubic: camera returned no frame yet");
            *warned_empty = true;
        }
        return;
    };

    if *frame_count == 0 {
        let (w, h) = frame.dimensions();
        eprintln!("rubic: receiving camera frames ({w}x{h})");
    }
    *frame_count += 1;

    // Detection is heavy and doesn't need every frame (~2/sec). Run it on a
    // cadence and reuse the last read for the preview between runs so the video
    // stays smooth.
    if *frame_count % DETECT_INTERVAL == 0 {
        *last_read = read_face_grid_detail(&frame);
        session.detected = last_read.is_some();
    }

    upload_preview(&frame, &mut images, &preview.0, last_read.as_ref());
}

/// Detect on roughly every Nth frame (~2/sec at 30 fps) rather than each tick.
const DETECT_INTERVAL: u64 = 15;

/// On [`CaptureEvent::Completed`], write the scan into the review state and
/// switch to Input mode.
fn finish_if_complete(
    event: CaptureEvent,
    session: &mut CameraSession,
    mode: &mut AppMode,
    input: &mut InputState,
) {
    if event == CaptureEvent::Completed {
        if let Some(classified) = session.flow.finish() {
            input.partial = handoff(&classified);
        }
        session.flow.reset();
        *mode = AppMode::Input;
    }
}

/// Perceptual point of a face's ideal scheme color.
fn scheme_point(face: Face) -> [f32; 3] {
    let c = sticker_rgb(face);
    perceptual_point([
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
    ])
}

/// Nearest scheme face color to a sampled sticker, for the live net preview.
/// (The final [`crate::vision::classify`] pass is relative/cluster-based; this
/// is a quick per-face approximation for instant feedback.)
fn nearest_scheme_face(sample: Rgb) -> Face {
    let p = perceptual_point(sample);
    Face::ALL
        .into_iter()
        .min_by(|&a, &b| {
            point_distance_sq(p, scheme_point(a))
                .partial_cmp(&point_distance_sq(p, scheme_point(b)))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(Face::U)
}

/// Guidance for a face: `(which face by center color, how to orient it)`.
///
/// The orientation cue is required: each face's stickers are filed into fixed
/// facelet slots, so the face must be held the right way up or its border
/// stickers land rotated. Derived from the core's facelet geometry (standard
/// URFDLB, white=U/green=F): side faces keep white up; the white face keeps
/// green toward the bottom; the yellow face keeps green toward the top.
fn face_hint(face: Face) -> (&'static str, &'static str) {
    match face {
        Face::U => ("WHITE face", "keep the GREEN side at the BOTTOM"),
        Face::R => ("RED face", "keep WHITE on top"),
        Face::F => ("GREEN face", "keep WHITE on top"),
        Face::D => ("YELLOW face", "keep the GREEN side at the TOP"),
        Face::L => ("ORANGE face", "keep WHITE on top"),
        Face::B => ("BLUE face", "keep WHITE on top"),
    }
}

/// Update the camera HUD with the current step, like a check-scanner: which
/// face to present, whether it's in view, and how to capture it.
pub fn update_camera_hud(
    mode: Res<AppMode>,
    session: Res<CameraSession>,
    mut hud: Query<(&mut Text, &mut Visibility), With<CameraHud>>,
) {
    // Only the banner shows while scanning; hidden otherwise so it never
    // overlaps the rest of the UI.
    let want_vis = if *mode == AppMode::Camera {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let text = if *mode == AppMode::Camera {
        match session.flow.current_target() {
            Some(face) => {
                let step = session.flow.captured_count() + 1;
                let (name, orient) = face_hint(face);
                let ready = if session.detected {
                    ">> Face in view - press ENTER to capture <<"
                } else {
                    "Line the cube face up inside the box"
                };
                format!(
                    "Scanning face {step} of 6:  show the {name}\n\
                     Hold it flat to the camera and {orient}.\n\
                     {ready}\n\
                     ENTER = capture    R = restart    Esc = cancel"
                )
            }
            None => "Scan complete.".to_string(),
        }
    } else {
        String::new()
    };
    for (mut t, mut vis) in &mut hud {
        if *vis != want_vis {
            *vis = want_vis;
        }
        if t.0 != text {
            t.0.clone_from(&text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::classify::classify;
    use rubic_core::{Completion, Face, Facelets};

    fn face_rgb(f: Face) -> [u8; 3] {
        let c = crate::colors::sticker_rgb(f);
        [
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
        ]
    }

    #[test]
    fn handoff_produces_reviewable_unique_cube() {
        let samples: [[u8; 3]; 54] = std::array::from_fn(|i| face_rgb(Facelets::SOLVED.get(i)));
        let classified = classify(&samples);
        let partial = handoff(&classified);
        // All non-center stickers are filled and the cube is uniquely solvable.
        assert_eq!(partial.known_count(), 48);
        assert!(matches!(partial.analyze(), Completion::Unique(_)));
    }
}
