//! On-screen touch controls for mode switching and solve playback.
//!
//! Phones have no keyboard, so each button injects the equivalent key press and
//! the existing keyboard handlers (`paint`/`solve`) do the work — one source of
//! truth, no duplicated logic. Buttons are shown per mode and the row wraps on
//! narrow screens. (Camera-scan controls live separately in `camera_scan`.)

use bevy::prelude::*;
use rubic_core::Completion;

use crate::mode::{AppMode, InputStage};
use crate::paint::InputState;

/// A touch control; its action is delivered by injecting [`TouchControl::key`].
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum TouchControl {
    NewGame,   // ChooseMethod/Solve: scramble a random cube to play  (G)
    Manual,    // ChooseMethod: start painting a cube by hand         (M)
    Camera,    // ChooseMethod: open the webcam and scan (feature)    (C)
    Solve,     // Editing: confirm the cube and enter Solve           (Enter)
    StartOver, // Editing: back to the method picker                  (Esc)
    Edit,      // Solve: back to painting                             (Tab)
    Beginner,  // Solve: beginner solver                              (1)
    Optimal,   // Solve: optimal solver                               (2)
    Prev,      // Solve: step back                                    (Left)
    Play,      // Solve: play / pause                                 (Space)
    Next,      // Solve: step forward                                 (Right)
}

impl TouchControl {
    /// The keyboard key this control stands in for.
    fn key(self) -> KeyCode {
        match self {
            TouchControl::NewGame => KeyCode::KeyG,
            TouchControl::Manual => KeyCode::KeyM,
            TouchControl::Camera => KeyCode::KeyC,
            TouchControl::Solve => KeyCode::Enter,
            TouchControl::StartOver => KeyCode::Escape,
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
            TouchControl::Manual => "Manual",
            TouchControl::Camera => "Camera",
            TouchControl::Solve => "Solve",
            TouchControl::StartOver => "Start over",
            TouchControl::Edit => "Edit",
            TouchControl::Beginner => "Beginner",
            TouchControl::Optimal => "Optimal",
            TouchControl::Prev => "< Prev",
            TouchControl::Play => "Play / Pause",
            TouchControl::Next => "Next >",
        }
    }

    /// A short sub-label for the method-picker buttons, explaining the method.
    fn hint(self) -> Option<&'static str> {
        match self {
            TouchControl::NewGame => Some("random cube"),
            TouchControl::Manual => Some("paint by hand"),
            TouchControl::Camera => Some("scan with webcam"),
            _ => None,
        }
    }

    const ALL: [TouchControl; 11] = [
        TouchControl::NewGame,
        TouchControl::Manual,
        TouchControl::Camera,
        TouchControl::Solve,
        TouchControl::StartOver,
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
                        // Stack the label above its (optional) hint, centered.
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
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
                    // Method-picker buttons carry a small grey hint under the
                    // label, so a first-time user knows what each method does.
                    if let Some(hint) = control.hint() {
                        b.spawn((
                            Text::new(hint),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.65, 0.68, 0.74)),
                        ));
                    }
                });
            }
        });
}

/// Whether the top-bar `control` is shown in the given app state. The method
/// picker offers the three setup methods; editing offers Solve + Start over;
/// Solve mode offers scramble/edit/solver/playback. Camera mode uses its own
/// (bottom) bar, so the top bar is empty there.
fn top_bar_shows(control: TouchControl, mode: AppMode, stage: InputStage) -> bool {
    use TouchControl::{Beginner, Camera, Edit, Manual, NewGame, Next, Optimal, Play, Prev, Solve, StartOver};
    match mode {
        AppMode::Input => match stage {
            // The `Camera` method is only functional with a camera feature; its
            // key handler is compiled out otherwise, so hide the button too.
            InputStage::ChooseMethod => {
                matches!(control, NewGame | Manual)
                    || (control == Camera && cfg!(feature = "camera"))
            }
            InputStage::Editing => matches!(control, Solve | StartOver),
        },
        AppMode::Solve => matches!(
            control,
            NewGame | Edit | Beginner | Optimal | Prev | Play | Next
        ),
        AppMode::Camera => false,
    }
}

/// Show the per-state top-bar controls (see [`top_bar_shows`]).
pub fn update_touch_controls(
    mode: Res<AppMode>,
    stage: Res<InputStage>,
    mut controls: Query<(&TouchControl, &mut Node)>,
) {
    for (control, mut node) in &mut controls {
        let want = if top_bar_shows(*control, *mode, *stage) {
            Display::Flex
        } else {
            Display::None
        };
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
        assert_eq!(TouchControl::Manual.label(), "Manual");
        assert_eq!(TouchControl::StartOver.label(), "Start over");
    }

    #[test]
    fn method_buttons_carry_hints() {
        assert!(TouchControl::NewGame.hint().is_some());
        assert!(TouchControl::Manual.hint().is_some());
        assert!(TouchControl::Camera.hint().is_some());
        assert!(TouchControl::Solve.hint().is_none());
        assert!(TouchControl::StartOver.hint().is_none());
    }

    #[test]
    fn method_picker_shows_shuffle_and_manual_not_solve() {
        use AppMode::Input;
        use InputStage::ChooseMethod;
        assert!(top_bar_shows(TouchControl::NewGame, Input, ChooseMethod));
        assert!(top_bar_shows(TouchControl::Manual, Input, ChooseMethod));
        assert!(!top_bar_shows(TouchControl::Solve, Input, ChooseMethod));
        assert!(!top_bar_shows(TouchControl::StartOver, Input, ChooseMethod));
    }

    #[test]
    fn editing_shows_solve_and_start_over() {
        use AppMode::Input;
        use InputStage::Editing;
        assert!(top_bar_shows(TouchControl::Solve, Input, Editing));
        assert!(top_bar_shows(TouchControl::StartOver, Input, Editing));
        assert!(!top_bar_shows(TouchControl::Manual, Input, Editing));
        assert!(!top_bar_shows(TouchControl::NewGame, Input, Editing));
    }

    #[test]
    fn camera_button_gated_on_feature() {
        let shown = top_bar_shows(TouchControl::Camera, AppMode::Input, InputStage::ChooseMethod);
        assert_eq!(shown, cfg!(feature = "camera"));
    }
}
