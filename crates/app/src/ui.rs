//! On-screen help and live status HUD.
//!
//! A static controls panel (top-left) plus a dynamic status line (bottom-left)
//! showing the validation state and, once a solve is loaded, the solver name
//! and step counter. Called by `main.rs`.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::mode::{AppMode, InputStage};
use crate::net::{NET_W, NetRoot};
use crate::paint::{InputState, input_status};
use crate::solve::SolvePlayer;
use crate::types::{CubeRes, DesktopOnly, OrbitCamera, StatusText};
use crate::validation::status_line;

/// Below this window width (px) the app is treated as "narrow" (phone): the
/// desktop-only reference text is hidden, since touch controls guide instead.
pub const NARROW_WIDTH: f32 = 720.0;

/// Compact keyboard-shortcut reference (desktop). Touch buttons cover the same
/// actions, so this stays short.
const HELP: &str = "\
drag: orbit  ·  wheel: zoom  ·  click: paint  ·  1-6: color
setup — G: shuffle  ·  M: manual  ·  C: camera  ·  Esc: start over
Enter: solve  ·  Tab: edit  ·  1/2: solver  ·  Space/N/P: play·step";

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
            // Wrap within the viewport instead of running off the right edge.
            max_width: Val::Vw(62.0),
            ..default()
        },
        StatusText,
    ));
}

/// Refresh the status line from the mode, cube state, input, and solve player.
pub fn update_status(
    mode: Res<AppMode>,
    stage: Res<InputStage>,
    cube: Res<CubeRes>,
    input: Res<InputState>,
    player: Res<SolvePlayer>,
    mut text: Query<&mut Text, With<StatusText>>,
) {
    let detail = match *mode {
        AppMode::Input => match *stage {
            InputStage::ChooseMethod => "choose a setup method".to_string(),
            InputStage::Editing => input_status(&input),
        },
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

/// Reflow for the window width: on phones, stack the net and palette centered
/// (net below the top control bar, palette above the bottom bar) and shrink the
/// 3D cube so nothing overlaps; on desktop, tuck the net + palette top-right.
#[allow(clippy::type_complexity)]
pub fn responsive_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    mode: Res<AppMode>,
    mut net: Query<&mut Node, (With<NetRoot>, Without<StatusText>)>,
    mut status: Query<&mut Node, (With<StatusText>, Without<NetRoot>)>,
    mut orbit: ResMut<OrbitCamera>,
    mut last_narrow: Local<Option<bool>>,
) {
    let Ok(win) = windows.single() else {
        return;
    };
    let w = win.width();
    let narrow = w < NARROW_WIDTH;

    for mut n in &mut net {
        if narrow {
            // Centered, clear below the top control bar. (The palette rides
            // along in the net's top-right corner.)
            n.right = Val::Auto;
            n.left = Val::Px(((w - NET_W) / 2.0).max(4.0));
            n.top = Val::Px(96.0);
        } else {
            n.left = Val::Auto;
            n.right = Val::Px(8.0);
            n.top = Val::Px(8.0);
        }
    }
    for mut s in &mut status {
        // On phones: top-left in Input/Camera (the bottom holds a button bar),
        // but bottom-left in Solve, where the bottom is clear and the top is
        // full of playback buttons. Desktop: always bottom-left.
        if narrow && *mode != AppMode::Solve {
            s.bottom = Val::Auto;
            s.top = Val::Px(8.0);
        } else {
            s.top = Val::Auto;
            s.bottom = Val::Px(8.0);
        }
    }

    // Shrink the cube on phones. Only on a state change, so it doesn't fight
    // the user's pinch/scroll zoom within a size.
    if *last_narrow != Some(narrow) {
        orbit.radius = if narrow {
            17.0
        } else {
            OrbitCamera::DEFAULT.radius
        };
        *last_narrow = Some(narrow);
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
