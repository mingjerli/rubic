//! On-screen touch controls for mode switching and solve playback.
//!
//! Phones have no keyboard, so each button injects the equivalent key press and
//! the existing keyboard handlers (`paint`/`solve`) do the work — one source of
//! truth, no duplicated logic. Buttons are shown per mode and the row wraps on
//! narrow screens. (Camera-scan controls live separately in `camera_scan`.)

use bevy::prelude::*;
use rubic_core::Completion;

use crate::layout::{
    BUTTON_BORDER, FrameLayout, TOP_BAR_GAP, TOP_BAR_TOP, TOP_BUTTON_FONT, TOP_BUTTON_PAD,
    TOP_HINT_FONT,
};
use crate::paint::InputState;

/// A touch control; its action is delivered by injecting [`TouchControl::key`].
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
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

    pub(crate) fn label(self) -> &'static str {
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
    pub(crate) fn hint(self) -> Option<&'static str> {
        match self {
            TouchControl::NewGame => Some("random cube"),
            TouchControl::Manual => Some("paint by hand"),
            TouchControl::Camera => Some("scan with webcam"),
            _ => None,
        }
    }

    pub(crate) const ALL: [TouchControl; 11] = [
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

/// Marker for the top control bar's container.
#[derive(Component)]
pub struct TopBarRoot;

/// Startup: spawn the mode/solve control bar (camera controls live at the
/// bottom). Placement comes from the [`FrameLayout`]; hidden buttons are
/// toggled per mode.
pub fn setup_touch_controls(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(TOP_BAR_TOP),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(TOP_BAR_GAP.x),
                row_gap: Val::Px(TOP_BAR_GAP.y),
                ..default()
            },
            TopBarRoot,
        ))
        .with_children(|row| {
            for control in TouchControl::ALL {
                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(TOP_BUTTON_PAD.x), Val::Px(TOP_BUTTON_PAD.y)),
                        border: UiRect::all(Val::Px(BUTTON_BORDER)),
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
                            font_size: TOP_BUTTON_FONT,
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
                                font_size: TOP_HINT_FONT,
                                ..default()
                            },
                            TextColor(Color::srgb(0.65, 0.68, 0.74)),
                        ));
                    }
                });
            }
        });
}

/// Place the top bar and show the controls the [`FrameLayout`] lists for this
/// state.
#[allow(clippy::type_complexity)]
pub fn update_touch_controls(
    layout: Res<FrameLayout>,
    mut bar: Query<&mut Node, (With<TopBarRoot>, Without<TouchControl>)>,
    mut controls: Query<(&TouchControl, &mut Node), Without<TopBarRoot>>,
) {
    for mut node in &mut bar {
        layout.top_bar.edges.apply(&mut node);
        if node.justify_content != layout.top_bar.justify {
            node.justify_content = layout.top_bar.justify;
        }
    }
    for (control, mut node) in &mut controls {
        let want = if layout.top_bar.controls.contains(control) {
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
}
