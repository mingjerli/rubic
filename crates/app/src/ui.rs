//! On-screen help and live status HUD.
//!
//! A static controls panel (top-left) plus a dynamic status line (bottom-left)
//! showing the validation state and, once a solve is loaded, the solver name
//! and step counter. Called by `main.rs`.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::mode::AppMode;
use crate::paint::{InputState, input_status};
use crate::solve::SolvePlayer;
use crate::types::{CubeRes, DesktopOnly, StatusText};
use crate::validation::status_line;

/// Below this window width (px) the app is treated as "narrow" (phone): the
/// desktop-only reference text is hidden, since touch controls guide instead.
pub const NARROW_WIDTH: f32 = 720.0;

/// Compact keyboard-shortcut reference (desktop). Touch buttons cover the same
/// actions, so this stays short.
const HELP: &str = "\
drag: orbit  ·  wheel: zoom  ·  click: paint  ·  1-6: color
Tab: Input/Solve  ·  Enter: solve  ·  1/2: solver  ·  Space/N/P: play·step";

/// Spawn the help panel and the (initially empty) status line.
pub fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new(HELP),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.75, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        DesktopOnly,
    ));

    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        StatusText,
    ));
}

/// Refresh the status line from the mode, cube state, input, and solve player.
pub fn update_status(
    mode: Res<AppMode>,
    cube: Res<CubeRes>,
    input: Res<InputState>,
    player: Res<SolvePlayer>,
    mut text: Query<&mut Text, With<StatusText>>,
) {
    let detail = match *mode {
        AppMode::Input => input_status(&input),
        // Detailed per-face scan progress is shown by the camera-scan HUD.
        AppMode::Camera => "scanning…".to_string(),
        AppMode::Solve => status_line(&cube.0),
    };
    let mut line = format!("{} · {}", mode.label(), detail);
    if *mode == AppMode::Solve {
        if let Some(p) = &player.player {
            let playing = if p.playing { "  (playing)" } else { "" };
            line.push_str(&format!("\n{}{playing}", p.hud()));
        }
    }
    for mut t in &mut text {
        if t.0 != line {
            t.0.clone_from(&line);
        }
    }
}

/// Hide desktop-only reference text on narrow (phone) screens, where touch
/// controls guide the user; show it on wider (desktop) windows.
pub fn responsive_help(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut panels: Query<&mut Visibility, With<DesktopOnly>>,
) {
    let Ok(win) = windows.single() else {
        return;
    };
    let want = if win.width() < NARROW_WIDTH {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut vis in &mut panels {
        if *vis != want {
            *vis = want;
        }
    }
}
