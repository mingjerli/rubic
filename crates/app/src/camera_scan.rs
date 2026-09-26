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

use crate::action::Action;
use crate::colors::sticker_rgb;
use crate::layout::{
    BUTTON_BORDER, CAMERA_BAR_BOTTOM, CAMERA_BAR_GAP, CAMERA_BUTTON_FONT, CAMERA_BUTTON_PAD,
    CORNER_MARGIN, FrameLayout, HUD_FONT_WIDE, HUD_GAP_ABOVE_PREVIEW, HUD_PAD, PREVIEW_ASPECT,
    PREVIEW_MAX_W, visibility,
};
use crate::mode::{AppMode, InputStage};
use crate::paint::{InputState, start_over};
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
/// native camera handle is not `Sync`. Starts empty; the camera is opened on
/// demand via the on-screen toggle so the browser permission prompt only
/// appears when the user asks for it.
pub struct CameraFeed(pub Option<Box<dyn CameraSource>>);

/// Open the platform camera (native webcam or browser `getUserMedia`), or
/// `None` if unavailable. Called by the camera on/off toggle.
#[must_use]
pub fn open_source() -> Option<Box<dyn CameraSource>> {
    #[cfg(all(feature = "camera-native", not(target_arch = "wasm32")))]
    {
        match crate::vision::native::NativeCamera::open_default() {
            Ok(cam) => return Some(Box::new(cam)),
            Err(e) => eprintln!("rubic: camera unavailable ({e})"),
        }
    }
    #[cfg(all(feature = "camera-web", target_arch = "wasm32"))]
    {
        match crate::vision::web_camera::WebCamera::open() {
            Ok(cam) => return Some(Box::new(cam)),
            Err(e) => eprintln!("rubic: web camera unavailable ({e})"),
        }
    }
    None
}

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
                width: Val::Px(PREVIEW_MAX_W),
                height: Val::Px(PREVIEW_MAX_W * PREVIEW_ASPECT),
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

/// Place, size and show the preview and its HUD banner from the
/// [`FrameLayout`].
pub fn apply_camera_layout(
    layout: Res<FrameLayout>,
    mut previews: Query<(&mut Node, &mut Visibility), (With<PreviewNode>, Without<CameraHud>)>,
    mut huds: Query<(&mut Node, &mut Visibility), (With<CameraHud>, Without<PreviewNode>)>,
) {
    for (mut node, mut vis) in &mut previews {
        if let Some(edges) = layout.preview {
            edges.apply(&mut node);
        }
        let want = visibility(layout.preview.is_some());
        if *vis != want {
            *vis = want;
        }
    }
    for (mut node, mut vis) in &mut huds {
        if let Some(edges) = layout.hud {
            edges.apply(&mut node);
        }
        let want = visibility(layout.hud.is_some());
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
            font_size: HUD_FONT_WIDE,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.97, 1.0)),
        TextLayout::new_with_justify(JustifyText::Center),
        Node {
            position_type: PositionType::Absolute,
            // Directly above the preview, same right edge and width.
            bottom: Val::Px(CORNER_MARGIN + PREVIEW_MAX_W * PREVIEW_ASPECT + HUD_GAP_ABOVE_PREVIEW),
            right: Val::Px(CORNER_MARGIN),
            width: Val::Px(PREVIEW_MAX_W),
            padding: UiRect::all(Val::Px(HUD_PAD)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.85)),
        Visibility::Hidden,
        CameraHud,
    ));
}

// --- Shared scan actions (driven by both keyboard and on-screen buttons) -----

/// Enter camera-scan mode, resetting the flow — only if a camera was opened.
fn start_scan(feed: &CameraFeed, mode: &mut AppMode, session: &mut CameraSession) {
    if feed.0.is_some() {
        session.flow.reset();
        *mode = AppMode::Camera;
    }
}

/// Discard all captured faces and return to the first face (net back to centers).
fn restart_scan(session: &mut CameraSession, input: &mut InputState) {
    session.flow.reset();
    session.last_event = CaptureEvent::Idle;
    input.partial = PartialFacelets::new();
}

/// Cancel the scan and return to the method picker (reseeding the solved
/// preview), releasing the camera device.
fn cancel_scan(
    feed: &mut CameraFeed,
    session: &mut CameraSession,
    mode: &mut AppMode,
    stage: &mut InputStage,
    input: &mut InputState,
) {
    session.flow.reset();
    session.last_event = CaptureEvent::Idle;
    feed.0 = None;
    *mode = AppMode::Input;
    start_over(stage, input);
}

/// Capture (or retake) the current face from the latest frame and fill the net.
/// Stays on the same face so it can be retaken until it looks right.
fn capture_face(feed: &mut CameraFeed, session: &mut CameraSession, input: &mut InputState) {
    let Some(src) = feed.0.as_mut() else { return };
    let Some(frame) = src.next_frame() else {
        return;
    };
    // Use the detected face; fall back to the centered grid so a capture always
    // succeeds even if detection missed this frame.
    let samples = read_face_grid(&frame).unwrap_or_else(|| capture_centered(&frame));
    let target = session.flow.current_target();
    session.last_event = session.flow.capture(samples);
    // Live net fill: paint the captured face onto the 2D net right away
    // (approximate scheme colors); the final classify refines it at the end.
    if let Some(face) = target {
        for (k, &s) in samples.iter().enumerate() {
            input.partial = input
                .partial
                .set(face.index() * 9 + k, nearest_scheme_face(s));
        }
    }
}

/// Move on to the next face; hands off to review (and turns the camera off)
/// after the sixth.
fn next_face(
    feed: &mut CameraFeed,
    session: &mut CameraSession,
    mode: &mut AppMode,
    stage: &mut InputStage,
    input: &mut InputState,
) {
    let event = session.flow.advance();
    session.last_event = event;
    finish_if_complete(feed, event, session, mode, stage, input);
}

/// `StartCamera` (from the method picker): open the webcam if needed and jump
/// straight into the guided scan. Only from the picker, so it can't discard an
/// in-progress cube.
pub fn enter_camera_scan(
    mut actions: EventReader<Action>,
    stage: Res<InputStage>,
    mut feed: NonSendMut<CameraFeed>,
    mut mode: ResMut<AppMode>,
    mut session: ResMut<CameraSession>,
    mut input: ResMut<InputState>,
) {
    for action in actions.read() {
        if *action != Action::StartCamera
            || *mode != AppMode::Input
            || *stage != InputStage::ChooseMethod
        {
            continue;
        }
        if feed.0.is_none() {
            feed.0 = open_source();
        }
        // Clear the solved preview so the net starts blank and fills in as each
        // face is captured — that filling net is the scan's progress view.
        if feed.0.is_some() {
            input.partial = PartialFacelets::new();
        }
        start_scan(&feed, &mut mode, &mut session);
    }
}

/// During a Scan: `Capture` (or retake) the current face, `NextFace` /
/// `PrevFace`, `RestartScan`, or `StartOver` to leave the Scan.
pub fn camera_scan_controls(
    mut actions: EventReader<Action>,
    mut feed: NonSendMut<CameraFeed>,
    mut session: ResMut<CameraSession>,
    mut mode: ResMut<AppMode>,
    mut stage: ResMut<InputStage>,
    mut input: ResMut<InputState>,
) {
    for action in actions.read() {
        if *mode != AppMode::Camera {
            continue;
        }
        match action {
            Action::StartOver => {
                cancel_scan(&mut feed, &mut session, &mut mode, &mut stage, &mut input);
            }
            Action::RestartScan => restart_scan(&mut session, &mut input),
            Action::Capture => capture_face(&mut feed, &mut session, &mut input),
            Action::NextFace => {
                next_face(&mut feed, &mut session, &mut mode, &mut stage, &mut input);
            }
            Action::PrevFace => session.flow.step_back(),
            _ => {}
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
/// switch to Input mode (Editing, so the filled net shows for review).
fn finish_if_complete(
    feed: &mut CameraFeed,
    event: CaptureEvent,
    session: &mut CameraSession,
    mode: &mut AppMode,
    stage: &mut InputStage,
    input: &mut InputState,
) {
    if event == CaptureEvent::Completed {
        if let Some(classified) = session.flow.finish() {
            input.partial = handoff(&classified);
        }
        session.flow.reset();
        // Scan done: hand off to review and turn the camera off (releasing the
        // device) so it isn't left running.
        feed.0 = None;
        *mode = AppMode::Input;
        *stage = InputStage::Editing;
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

/// The HUD's one-line capture status for the current face.
fn hud_status(captured: bool, detected: bool) -> &'static str {
    if captured {
        "Captured ✓ — Next when happy"
    } else if detected {
        "In view — Capture now"
    } else {
        "Line the face up in the box"
    }
}

/// The HUD text for face `index` (0-based) of the scan. Keyboard hints only
/// make sense on desktop; the buttons carry those actions in the compact
/// layout.
fn hud_text(face: Face, index: usize, status: &str, compact: bool) -> String {
    let (name, orient) = face_hint(face);
    let mut s = format!("Face {}/6: {name}\n{orient}\n{status}", index + 1);
    if !compact {
        s.push_str("\nENTER capture/retake · N next · P prev · R restart · Esc cancel");
    }
    s
}

/// The longest HUD text any face can show, for layout sizing.
#[cfg(test)]
pub fn longest_hud_text(compact: bool) -> String {
    let statuses = [
        hud_status(true, false),
        hud_status(false, true),
        hud_status(false, false),
    ];
    Face::ALL
        .into_iter()
        .flat_map(|face| statuses.map(|status| hud_text(face, 5, status, compact)))
        .max_by_key(|text| text.lines().map(|l| l.chars().count()).max())
        .unwrap_or_default()
}

/// Update the camera HUD with the current step, like a check-scanner: which
/// face to present, whether it's in view, and how to capture it.
///
/// On phones the text is smaller and terser and the keyboard-shortcut line is
/// dropped (the on-screen buttons cover those actions), so the banner stays
/// compact and doesn't crowd the progress net.
pub fn update_camera_hud(
    mode: Res<AppMode>,
    session: Res<CameraSession>,
    layout: Res<FrameLayout>,
    mut hud: Query<(&mut Text, &mut TextFont), With<CameraHud>>,
) {
    let text = if *mode == AppMode::Camera {
        match session.flow.current_target() {
            Some(face) => hud_text(
                face,
                session.flow.current_index(),
                hud_status(session.flow.current_captured(), session.detected),
                layout.compact,
            ),
            None => "Scan complete.".to_string(),
        }
    } else {
        String::new()
    };
    let font_size = layout.hud_font;
    for (mut t, mut f) in &mut hud {
        if (f.font_size - font_size).abs() > f32::EPSILON {
            f.font_size = font_size;
        }
        if t.0 != text {
            t.0.clone_from(&text);
        }
    }
}

// --- On-screen touch controls (phones/tablets have no keyboard) -------------

/// A tappable control button; the variant is the action it performs. Entering
/// the scan is driven from the top-bar `Camera` method, so the bottom bar holds
/// only the in-scan controls.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CamButton {
    Prev,
    Capture,
    Next,
    Restart,
    Back,
}

impl CamButton {
    /// Display order in the bar.
    pub const ALL: [CamButton; 5] = [
        CamButton::Prev,
        CamButton::Capture,
        CamButton::Next,
        CamButton::Restart,
        CamButton::Back,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CamButton::Prev => "< Prev",
            CamButton::Capture => "Capture / Retake",
            CamButton::Next => "Next >",
            CamButton::Restart => "Restart",
            CamButton::Back => "Start over",
        }
    }

    /// What tapping this button asks for.
    #[must_use]
    pub fn action(self) -> Action {
        match self {
            CamButton::Prev => Action::PrevFace,
            CamButton::Capture => Action::Capture,
            CamButton::Next => Action::NextFace,
            CamButton::Restart => Action::RestartScan,
            CamButton::Back => Action::StartOver,
        }
    }

    fn color(self) -> Color {
        match self {
            CamButton::Prev | CamButton::Back => Color::srgb(0.35, 0.35, 0.42),
            CamButton::Capture => Color::srgb(0.15, 0.60, 0.30),
            CamButton::Next => Color::srgb(0.20, 0.50, 0.90),
            CamButton::Restart => Color::srgb(0.55, 0.45, 0.15),
        }
    }
}

/// Startup: spawn the touch control buttons in a bottom-center row. Each is
/// shown only where it applies (see [`update_camera_buttons`]).
pub fn setup_camera_buttons(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(CAMERA_BAR_BOTTOM),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(CAMERA_BAR_GAP.x),
                row_gap: Val::Px(CAMERA_BAR_GAP.y),
                ..default()
            },
            CamButtonBar,
        ))
        .with_children(|row| {
            for action in CamButton::ALL {
                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(
                            Val::Px(CAMERA_BUTTON_PAD.x),
                            Val::Px(CAMERA_BUTTON_PAD.y),
                        ),
                        border: UiRect::all(Val::Px(BUTTON_BORDER)),
                        // Hidden via display so it reserves no layout space.
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(action.color()),
                    BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                    action,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new(action.label()),
                        TextFont {
                            font_size: CAMERA_BUTTON_FONT,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
}

/// Marker for the bottom control bar, so it can be sized around the preview.
#[derive(Component)]
pub struct CamButtonBar;

/// Place the control bar from the [`FrameLayout`] (left of the preview while
/// the camera is on, so no button hides under it).
pub fn layout_camera_bar(layout: Res<FrameLayout>, mut bar: Query<&mut Node, With<CamButtonBar>>) {
    for mut node in &mut bar {
        layout.camera_bar.apply(&mut node);
    }
}

/// Show the in-scan capture controls only while scanning; the bottom bar is
/// empty otherwise (entering the scan is a top-bar method).
pub fn update_camera_buttons(
    layout: Res<FrameLayout>,
    mut buttons: Query<(&CamButton, &mut Node)>,
) {
    for (_action, mut node) in &mut buttons {
        let want = if layout.camera_buttons {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
    }
}

/// Camera-bar adapter: a tapped button emits its [`Action`].
pub fn camera_button_actions(
    interactions: Query<(&Interaction, &CamButton), Changed<Interaction>>,
    mut actions: EventWriter<Action>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            actions.write(button.action());
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
