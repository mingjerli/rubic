//! On-screen help and live status HUD.
//!
//! A static controls panel (top-left) plus a dynamic status line (bottom-left)
//! showing the validation state and, once a solve is loaded, the solver name
//! and step counter. Called by `main.rs`.

use bevy::prelude::*;

use crate::solve::SolvePlayer;
use crate::types::{CubeRes, StatusText};
use crate::validation::status_line;

/// Static controls reference.
const HELP: &str = "\
rubic - interactive Rubik's Cube
--------------------------------
Camera:  left-drag orbit | right/middle-drag pan | wheel zoom | Home/0 reset
Play:    U D L R F B turn face  (+Shift = counter-clockwise)
Reset:   Backspace -> solved
Solve:   1 beginner | 2 optimal
Step:    Space play/pause | Right/N next | Left/P prev";

/// Spawn the help panel and the (initially empty) status line.
pub fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new(HELP),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.88, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
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

/// Refresh the status line from the cube state and solve player.
pub fn update_status(
    cube: Res<CubeRes>,
    player: Res<SolvePlayer>,
    mut text: Query<&mut Text, With<StatusText>>,
) {
    let mut line = format!("Status: {}", status_line(&cube.0));
    if let Some(p) = &player.player {
        line.push('\n');
        line.push_str(&p.hud());
        if p.playing {
            line.push_str("  (playing)");
        }
    }
    for mut t in &mut text {
        if t.0 != line {
            t.0.clone_from(&line);
        }
    }
}
