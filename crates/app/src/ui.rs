//! On-screen help and live status HUD.
//!
//! A static controls panel (top-left) plus a dynamic status line (bottom-left)
//! showing the validation state and, once a solve is loaded, the solver name
//! and step counter. Placement comes from the [`FrameLayout`]. Called by
//! `main.rs`.

use bevy::prelude::*;

use crate::layout::{
    CubeFraming, EDGE, FrameLayout, HELP_FONT, STATUS_FONT, STATUS_MAX_VW, visibility,
};
use crate::mode::{AppMode, InputStage};
use crate::net::NetRoot;
use crate::paint::{InputState, input_status};
use crate::solve::SolvePlayer;
use crate::types::{CubeRes, DesktopOnly, OrbitCamera, StatusText};
use crate::validation::status_line;

/// Compact keyboard-shortcut reference (desktop). Touch buttons cover the same
/// actions, so this stays short.
/// ASCII only: the default font has no glyphs for `·` or `—`.
pub(crate) const HELP: &str = "\
drag: orbit | wheel: zoom | click: paint | 1-6: color
setup - G: shuffle | M: manual | C: camera | Esc: start over
Enter: solve | Tab: edit | 1/2: solver | Space/N/P: play/step";

/// Spawn the help panel and the (initially empty) status line.
pub fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new(HELP),
        TextFont {
            font_size: HELP_FONT,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.75, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(EDGE),
            left: Val::Px(EDGE),
            ..default()
        },
        DesktopOnly,
    ));

    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: STATUS_FONT,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(EDGE),
            left: Val::Px(EDGE),
            // Wrap within the viewport instead of running off the right edge.
            max_width: Val::Vw(STATUS_MAX_VW),
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

/// Place the net and status line from the [`FrameLayout`], and frame the cube.
///
/// The cube framing is only re-applied when it changes, so it doesn't fight the
/// user's zoom / orbit within a layout.
#[allow(clippy::type_complexity)]
pub fn apply_layout(
    layout: Res<FrameLayout>,
    mut net: Query<&mut Node, (With<NetRoot>, Without<StatusText>)>,
    mut status: Query<&mut Node, (With<StatusText>, Without<NetRoot>)>,
    mut orbit: ResMut<OrbitCamera>,
    mut last_framing: Local<Option<CubeFraming>>,
) {
    if let Some(edges) = layout.net {
        for mut n in &mut net {
            edges.apply(&mut n);
        }
    }
    for mut s in &mut status {
        layout.status.apply(&mut s);
    }
    if let Some(framing) = layout.cube {
        if *last_framing != Some(framing) {
            orbit.radius = framing.radius;
            orbit.focus = Vec3::new(0.0, framing.focus_y, 0.0);
            *last_framing = Some(framing);
        }
    }
}

/// Show the desktop-only reference text only when the layout calls for it.
pub fn apply_desktop_text(
    layout: Res<FrameLayout>,
    mut panels: Query<&mut Visibility, With<DesktopOnly>>,
) {
    let want = visibility(layout.desktop_text);
    for mut vis in &mut panels {
        if *vis != want {
            *vis = want;
        }
    }
}
