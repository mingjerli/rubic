//! On-screen touch controls for mode switching and solve playback.
//!
//! Phones have no keyboard, so each button injects the equivalent key press and
//! the existing keyboard handlers (`paint`/`solve`) do the work — one source of
//! truth, no duplicated logic. Buttons are shown per mode and the row wraps on
//! narrow screens. (Camera-scan controls live separately in `camera_scan`.)

use bevy::prelude::*;

use crate::mode::AppMode;

/// A touch control; its action is delivered by injecting [`TouchControl::key`].
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum TouchControl {
    Solve,    // Input: confirm the cube and enter Solve  (Enter)
    Edit,     // Solve: back to painting                  (Tab)
    Beginner, // Solve: beginner solver                   (1)
    Optimal,  // Solve: optimal solver                    (2)
    Prev,     // Solve: step back                         (Left)
    Play,     // Solve: play / pause                      (Space)
    Next,     // Solve: step forward                      (Right)
}

impl TouchControl {
    /// The keyboard key this control stands in for.
    fn key(self) -> KeyCode {
        match self {
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
            TouchControl::Solve => "Solve this",
            TouchControl::Edit => "Edit",
            TouchControl::Beginner => "Beginner",
            TouchControl::Optimal => "Optimal",
            TouchControl::Prev => "< Prev",
            TouchControl::Play => "Play / Pause",
            TouchControl::Next => "Next >",
        }
    }

    const ALL: [TouchControl; 7] = [
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
            AppMode::Input => *control == TouchControl::Solve,
            AppMode::Solve => *control != TouchControl::Solve,
            AppMode::Camera => false,
        };
        let want = if show { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
}

/// Inject the matching key press for a tapped control, so the existing keyboard
/// handlers perform the action. Must be ordered before those handlers.
pub fn touch_control_input(
    interactions: Query<(&Interaction, &TouchControl), Changed<Interaction>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    for (interaction, control) in &interactions {
        if *interaction == Interaction::Pressed {
            keys.press(control.key());
        }
    }
}
