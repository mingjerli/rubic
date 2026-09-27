//! Bevy wiring for camera cube input (spec 0002, Phase B).
//!
//! This feeds face readings from a live [`CameraSource`] into the Scan in
//! progress ([`crate::scan::Scan`], tested there and through the Flow) and
//! renders it. Opening and closing the camera are Flow Effects. A real camera
//! and display are needed to exercise this wiring, so it is validated
//! on-device.
//!
//! A live video preview streams each camera frame into a fixed-size texture
//! ([`setup_camera_preview`] / [`upload_preview`]) shown while scanning, plus a
//! text HUD for the target face and progress.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::{RgbImage, imageops};

use crate::action::Action;
use crate::flow::Flow;
use crate::layout::{
    BUTTON_BORDER, CAMERA_BAR_BOTTOM, CAMERA_BAR_GAP, CAMERA_BUTTON_FONT, CAMERA_BUTTON_PAD,
    CORNER_MARGIN, FrameLayout, HUD_FONT_WIDE, HUD_GAP_ABOVE_PREVIEW, HUD_PAD, PREVIEW_ASPECT,
    PREVIEW_MAX_W, visibility,
};
use crate::scan::hud::hud_text;
use crate::vision::Rgb;
use crate::vision::pipeline::read_face_grid_detail;
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

/// Every tick, pull a frame into the live preview and tell the Scan whether an
/// image arrived. On a detection cadence, read the face in view and hand the
/// reading to the Scan, so the preview shows the read colors and a Capture
/// commits that reading (capture is manual, like lining a check up before
/// snapping it).
pub fn pump_camera(
    mut feed: NonSendMut<CameraFeed>,
    mut flow: ResMut<Flow>,
    time: Res<Time>,
    preview: Res<PreviewImage>,
    mut images: ResMut<Assets<Image>>,
    mut frame_count: Local<u64>,
    mut last_read: Local<Option<FaceRead>>,
) {
    let Some(src) = feed.0.as_mut() else {
        return;
    };
    let frame = src.next_frame();
    if let Some(scan) = flow.scan_mut() {
        scan.tick(time.elapsed_secs(), frame.is_some());
    }
    let Some(frame) = frame else {
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
        if let Some(scan) = flow.scan_mut() {
            scan.observe(last_read.map(|(colors, _)| colors));
        }
    }

    upload_preview(&frame, &mut images, &preview.0, last_read.as_ref());
}

/// Detect on roughly every Nth frame (~2/sec at 30 fps) rather than each tick.
const DETECT_INTERVAL: u64 = 15;

/// Update the camera HUD with the current step, like a check-scanner: which
/// face to present, whether it's in view, and how to capture it.
///
/// On phones the text is smaller and terser and the keyboard-shortcut line is
/// dropped (the on-screen buttons cover those actions), so the banner stays
/// compact and doesn't crowd the progress net.
pub fn update_camera_hud(
    flow: Res<Flow>,
    layout: Res<FrameLayout>,
    mut hud: Query<(&mut Text, &mut TextFont), With<CameraHud>>,
) {
    let text = flow
        .scan()
        .map(|scan| hud_text(scan, layout.compact))
        .unwrap_or_default();
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
mod tests;
