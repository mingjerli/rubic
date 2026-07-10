//! On-screen touch controls for mode switching and solve playback.
//!
//! Phones have no keyboard, so each button injects the equivalent key press and
//! the existing keyboard handlers (`paint`/`solve`) do the work — one source of
//! truth, no duplicated logic. Buttons are shown per mode and the row wraps on
//! narrow screens. (Camera-scan controls live separately in `camera_scan`.)

use bevy::prelude::*;
use rubic_core::Completion;

use crate::mode::AppMode;
use crate::paint::InputState;

/// A touch control; its action is delivered by injecting [`TouchControl::key`].
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum TouchControl {
    NewGame,  // Input/Solve: scramble a random cube to play  (G)
    Solve,    // Input: confirm the cube and enter Solve       (Enter)
    Edit,     // Solve: back to painting                       (Tab)
    Beginner, // Solve: beginner solver                        (1)
    Optimal,  // Solve: optimal solver                         (2)
    Prev,     // Solve: step back                              (Left)
    Play,     // Solve: play / pause                           (Space)
    Next,     // Solve: step forward                           (Right)
}

impl TouchControl {
    /// The keyboard key this control stands in for.
    fn key(self) -> KeyCode {
        match self {
            TouchControl::NewGame => KeyCode::KeyG,
            TouchControl::Solve => KeyCode::Enter,
            TouchControl::Edit => KeyCode::Tab,
            TouchControl::Beginner => KeyCode::Digit1,
            TouchControl::Optimal => KeyCode::Digit2,
            TouchControl::Prev => KeyCode::ArrowLeft,
            TouchControl::Play => KeyCode::Space,
            TouchControl::Next => KeyCode::ArrowRight,
        }
    }

    fn label(self) -> &'static str {
        match self {
            TouchControl::NewGame => "Shuffle",
            TouchControl::Solve => "Solve",
            TouchControl::Edit => "Edit",
            TouchControl::Beginner => "Beginner",
            TouchControl::Optimal => "Optimal",
            TouchControl::Prev => "< Prev",
            TouchControl::Play => "Play / Pause",
            TouchControl::Next => "Next >",
        }
    }

    const ALL: [TouchControl; 8] = [
        TouchControl::NewGame,
        TouchControl::Solve,
        TouchControl::Edit,
        TouchControl::Beginner,
        TouchControl::Optimal,
        TouchControl::Prev,
        TouchControl::Play,
        TouchControl::Next,
    ];
}

/// Startup: spawn the mode/solve control bar at the top-center (camera controls
/// live at the bottom). Hidden buttons are toggled per mode.
pub fn setup_touch_controls(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            // Below the (desktop) help panel so they never overlap; harmless
            // gap on mobile where the help is hidden.
            top: Val::Px(48.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            for control in TouchControl::ALL {
                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        // Hidden via display so it reserves no layout space; the
                        // visible buttons then center properly.
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.16, 0.18, 0.24, 0.92)),
                    BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
                    control,
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new(control.label()),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
}

/// Show `Solve` in Input mode and the playback controls in Solve mode; hide all
/// while camera-scanning.
pub fn update_touch_controls(mode: Res<AppMode>, mut controls: Query<(&TouchControl, &mut Node)>) {
    for (control, mut node) in &mut controls {
        let show = match *mode {
            AppMode::Input => matches!(control, TouchControl::Solve | TouchControl::NewGame),
            // Solve mode: everything except the Input-only "Solve this" (so
            // New game, Edit, solvers, and playback all show).
            AppMode::Solve => *control != TouchControl::Solve,
            AppMode::Camera => false,
        };
        let want = if show { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
}

/// Whether the entered cube is ready to solve: only a uniquely-determined state
/// can be confirmed into Solve mode (see [`crate::paint::mode_control`]).
#[must_use]
pub fn solve_ready(completion: &Completion) -> bool {
    matches!(completion, Completion::Unique(_))
}

/// Accent (ready) and dimmed (not-ready) styling for the `Solve` button.
const SOLVE_READY_BG: Color = Color::srgb(0.15, 0.60, 0.30);
const SOLVE_READY_FG: Color = Color::WHITE;
const SOLVE_DIM_BG: Color = Color::srgba(0.16, 0.18, 0.24, 0.55);
const SOLVE_DIM_FG: Color = Color::srgb(0.5, 0.53, 0.6);

/// Style the `Solve` button by input readiness (Input mode only): accent green
/// when the painted/scanned cube is uniquely solvable, dimmed otherwise, so the
/// goal is always visible but clearly inert until the cube is complete.
pub fn style_solve_button(
    input: Res<InputState>,
    mut buttons: Query<(&TouchControl, &mut BackgroundColor, &Children)>,
    mut texts: Query<&mut TextColor>,
) {
    let (bg, fg) = if solve_ready(&input.completion()) {
        (SOLVE_READY_BG, SOLVE_READY_FG)
    } else {
        (SOLVE_DIM_BG, SOLVE_DIM_FG)
    };
    for (control, mut background, children) in &mut buttons {
        if *control != TouchControl::Solve {
            continue;
        }
        if background.0 != bg {
            background.0 = bg;
        }
        for &child in children {
            if let Ok(mut color) = texts.get_mut(child) {
                if color.0 != fg {
                    color.0 = fg;
                }
            }
        }
    }
}

/// Inject the matching key press for a tapped control, so the existing keyboard
/// handlers perform the action. Must be ordered before those handlers.
///
/// An injected press is never released by winit (there's no real key), so we
/// release the previous frame's injections here first. Without this the key
/// stays "held", `just_pressed` never fires again, and a button stops
/// responding after its first tap.
pub fn touch_control_input(
    interactions: Query<(&Interaction, &TouchControl), Changed<Interaction>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut injected: Local<Vec<KeyCode>>,
) {
    for key in injected.drain(..) {
        keys.release(key);
    }
    for (interaction, control) in &interactions {
        if *interaction == Interaction::Pressed {
            let key = control.key();
            keys.press(key);
            injected.push(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubic_core::{Facelets, PartialFacelets};

    #[test]
    fn solve_ready_only_for_unique() {
        // A fully painted solved cube is uniquely determined -> ready.
        let unique = PartialFacelets::from_facelets(&Facelets::SOLVED).analyze();
        assert!(solve_ready(&unique));

        // Centers-only (nothing painted) needs more input -> not ready.
        let need_more = PartialFacelets::new().analyze();
        assert!(!solve_ready(&need_more));
    }

    #[test]
    fn labels_use_clear_verbs() {
        assert_eq!(TouchControl::NewGame.label(), "Shuffle");
        assert_eq!(TouchControl::Solve.label(), "Solve");
    }
}
