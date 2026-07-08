//! Bevy wiring for camera cube input (spec 0002, Phase B).
//!
//! **Compile-verified only.** This drives the tested vision pipeline
//! ([`crate::vision`]) from a live [`CameraSource`], but a real camera and
//! display are needed to exercise it, so behavior here is validated on-device,
//! not in tests. The one piece of pure logic — handing a completed scan to the
//! paint-review state — is unit-tested below.
//!
//! Live video *preview* (streaming frames into a GPU texture) is intentionally
//! deferred to on-hardware work: an unrunnable texture pipeline is exactly the
//! fragile code to avoid writing blind. The scan shows a text HUD instead
//! (target face + progress); everything else — capture, classify, hand-off — is
//! wired here.

use bevy::prelude::*;
use rubic_core::PartialFacelets;

use crate::mode::AppMode;
use crate::paint::InputState;
use crate::vision::capture::{CaptureEvent, CaptureFlow};
use crate::vision::classify::Classified;
use crate::vision::pipeline::capture_from_frame;
use crate::vision::source::CameraSource;

/// The in-progress camera scan.
#[derive(Resource, Default)]
pub struct CameraSession {
    /// The guided capture state machine.
    pub flow: CaptureFlow,
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

/// Startup: spawn the (initially hidden) camera-scan HUD text.
pub fn setup_camera_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.9, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(90.0),
            left: Val::Px(8.0),
            ..default()
        },
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

/// In camera mode, `Escape` or `Tab` returns to Input; `Space` force-captures
/// the current target from the latest frame.
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
    if keys.just_pressed(KeyCode::Space) {
        if let Some(src) = feed.0.as_mut() {
            if let Some(frame) = src.next_frame() {
                if let Some(samples) = capture_from_frame(&frame) {
                    let event = session.flow.force_capture(samples);
                    finish_if_complete(event, &mut session, &mut mode, &mut input);
                }
            }
        }
    }
}

/// Each tick, pull a frame, feed the capture flow, and hand off when complete.
pub fn run_camera_scan(
    mut feed: NonSendMut<CameraFeed>,
    mut session: ResMut<CameraSession>,
    mut mode: ResMut<AppMode>,
    mut input: ResMut<InputState>,
) {
    let Some(src) = feed.0.as_mut() else {
        return;
    };
    let Some(frame) = src.next_frame() else {
        return;
    };
    let detected = capture_from_frame(&frame);
    let event = session.flow.on_frame(detected);
    finish_if_complete(event, &mut session, &mut mode, &mut input);
}

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

/// Update the camera HUD text with the target face and progress.
pub fn update_camera_hud(
    mode: Res<AppMode>,
    session: Res<CameraSession>,
    mut hud: Query<&mut Text, With<CameraHud>>,
) {
    let text = if *mode == AppMode::Camera {
        match session.flow.current_target() {
            Some(face) => format!(
                "Scanning: show the {} face  ({}/6 captured)\n\
                 Space = capture now | Esc/Tab = back",
                face.to_char(),
                session.flow.captured_count()
            ),
            None => "Scan complete.".to_string(),
        }
    } else {
        String::new()
    };
    for mut t in &mut hud {
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
