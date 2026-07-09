//! `rubic` - an interactive Bevy GUI for the `rubic-core` Rubik's Cube library.
//!
//! Run with no arguments to launch the GUI on a solved cube, or seed the start
//! state with `--scramble "R U R' U2"` / `--facelets "<54 chars>"`.
//!
//! # Module map
//! - [`cli`]        argument parsing + initial-state construction (pure).
//! - [`colors`]     face -> sticker color (pure).
//! - [`geometry`]   facelet layout + layer/rotation math (pure).
//! - [`types`]      shared ECS components and resources.
//! - [`cube_render`] spawn + sync the 3D cube.
//! - [`camera`]     orbit / pan / zoom camera.
//! - [`input`]      manual face turns + reset.
//! - [`animation`]  layer-turn animation, driving state changes.
//! - [`solve`]      solvers + step playback.
//! - [`ui`]         on-screen help and status HUD.
//! - [`validation`] cube validity summary for the HUD (pure).
//!
//! # Deferred / partial
//! - Click-to-paint sticker entry (FR3) is deferred; the start state is set via
//!   the CLI and the live validation HUD reports Solved / Valid / Invalid. All
//!   other FRs (render, camera, manual play, solve + animate + step) are wired.
//! - Touch input (FR7 mobile) is deferred; see `camera.rs`.

// Bevy's system signatures trip several pedantic lints constantly, so this app
// crate opts out of them rather than inheriting the workspace `pedantic` set.
// The default `clippy::all` stays clean, and `unsafe` remains forbidden.
#![allow(
    clippy::pedantic,
    clippy::needless_pass_by_value,
    clippy::type_complexity,
    clippy::too_many_arguments
)]
#![forbid(unsafe_code)]

mod animation;
mod axis;
mod camera;
#[cfg(feature = "camera")]
mod camera_scan;
mod cli;
mod colors;
mod cube_render;
mod game;
mod geometry;
mod input;
mod mode;
mod net;
mod paint;
mod play;
mod solve;
mod touch;
mod types;
mod ui;
mod validation;
// Computer-vision core for camera cube input (spec 0002), behind the `camera`
// feature. Named `vision` to avoid colliding with the orbit-`camera` module.
#[cfg(feature = "camera")]
mod vision;

use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use clap::Parser;

use crate::cli::{Cli, Command};
use crate::mode::{AppMode, in_input, in_solve};
use crate::paint::InputState;
use crate::solve::SolvePlayer;
use crate::types::{CubeRes, OrbitCamera, TurnQueue};

fn main() {
    let cli = Cli::parse();

    // Non-GUI subcommands run and exit before Bevy starts.
    if let Some(Command::Cheatsheet { markdown, output }) = &cli.command {
        if let Err(message) = cli::run_cheatsheet(*markdown, output.as_ref()) {
            eprintln!("rubic: {message}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(Command::CaptureDebug) = &cli.command {
        capture_debug();
        return;
    }

    let facelets = match cli::initial_facelets(&cli) {
        Ok(f) => f,
        Err(message) => {
            eprintln!("rubic: {message}");
            std::process::exit(1);
        }
    };

    // Start painting a blank cube unless the CLI supplied a starting state.
    let input_state = if cli.scramble.is_some() || cli.facelets.is_some() {
        InputState::seeded(&facelets)
    } else {
        InputState::empty()
    };

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "rubic - Rubik's Cube".to_string(),
                // On the web, size the canvas to its parent (the page body) so
                // it fits the viewport instead of a fixed desktop resolution —
                // essential on phones. Harmless on native.
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }),
        MeshPickingPlugin,
    ))
    .insert_resource(ClearColor(Color::srgb(0.10, 0.11, 0.13)))
    .insert_resource(CubeRes(facelets))
    .insert_resource(input_state)
    .init_resource::<AppMode>()
    .init_resource::<TurnQueue>()
    .init_resource::<OrbitCamera>()
    .init_resource::<SolvePlayer>()
    .init_resource::<play::OrbitSuppressed>()
    .add_observer(paint::on_sticker_click)
    .add_observer(play::on_drag_start)
    .add_observer(play::on_drag_end)
    .add_systems(
        Startup,
        (
            camera::setup_camera,
            cube_render::setup_cube,
            ui::setup_ui,
            solve::setup_solvers,
            net::setup_net,
            axis::setup_legend,
            touch::setup_touch_controls,
        ),
    )
    // Always-on: camera, mode switching, net + axis reference, HUD, the
    // animation driver (which repaints from CubeRes when a turn lands), and the
    // touch controls (visibility + tap dispatch).
    .add_systems(
        Update,
        (
            camera::orbit_camera,
            paint::mode_control,
            game::scramble_input,
            net::net_render,
            net::toggle_input_ui,
            axis::draw_axes,
            ui::update_status,
            ui::responsive_help,
            ui::responsive_layout,
            touch::update_touch_controls,
            (animation::drive_turns, cube_render::sync_stickers).chain(),
        ),
    )
    // A tapped touch control injects the matching key press, so it must run
    // before the keyboard handlers that consume it.
    .add_systems(
        Update,
        touch::touch_control_input
            .before(paint::mode_control)
            .before(solve::solve_input)
            .before(solve::player_controls)
            .before(game::scramble_input),
    )
    // Solve mode: manual turns, solving, and step playback.
    .add_systems(
        Update,
        (
            input::manual_input,
            solve::solve_input,
            solve::player_controls,
            solve::auto_advance,
        )
            .run_if(in_solve),
    )
    // Input mode: paint the cube (net + 3D), select colors, live preview.
    .add_systems(
        Update,
        (
            paint::palette_keys,
            net::net_click,
            net::palette_click,
            paint::sync_input_stickers,
        )
            .run_if(in_input),
    );

    // Camera cube input (spec 0002), behind the `camera` feature.
    #[cfg(feature = "camera")]
    {
        app.init_resource::<camera_scan::CameraSession>()
            // Camera starts off; the on-screen toggle opens it on demand.
            .insert_non_send_resource(camera_scan::CameraFeed(None))
            .add_systems(
                Startup,
                (
                    camera_scan::setup_camera_hud,
                    camera_scan::setup_camera_preview,
                    camera_scan::setup_camera_buttons,
                ),
            )
            // Preview, frame pump, HUD, and touch buttons run every tick so the
            // live feed always shows, the HUD hides itself outside camera mode,
            // and the on-screen buttons work without a keyboard.
            .add_systems(
                Update,
                (
                    camera_scan::toggle_preview,
                    camera_scan::resize_preview,
                    camera_scan::pump_camera,
                    camera_scan::update_camera_hud,
                    camera_scan::update_camera_buttons,
                    camera_scan::update_camera_toggle_label,
                    camera_scan::camera_button_input,
                ),
            )
            .add_systems(Update, camera_scan::enter_camera_scan.run_if(in_input))
            .add_systems(
                Update,
                camera_scan::camera_scan_controls.run_if(crate::mode::in_camera),
            );
    }

    app.run();
}

/// Capture one frame and dump detection debug images to `/tmp`.
fn capture_debug() {
    #[cfg(all(feature = "camera-native", not(target_arch = "wasm32")))]
    capture_debug_native();
    #[cfg(not(all(feature = "camera-native", not(target_arch = "wasm32"))))]
    eprintln!("rubic: capture-debug needs a `--features camera-native` build");
}

#[cfg(all(feature = "camera-native", not(target_arch = "wasm32")))]
fn capture_debug_native() {
    use crate::vision::detect::{detect_stickers, draw_quad};
    use crate::vision::pipeline::read_face_grid_detail;
    use crate::vision::source::CameraSource;

    let mut cam = match crate::vision::native::NativeCamera::open_default() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rubic: camera error: {e}");
            return;
        }
    };
    // Warm up so auto-exposure/white-balance settle before grabbing.
    let mut frame = None;
    for _ in 0..20 {
        frame = cam.next_frame();
    }
    let Some(frame) = frame else {
        eprintln!("rubic: no frame captured");
        return;
    };

    let (w, h) = frame.dimensions();
    let _ = frame.save("/tmp/rubic-cam.png");
    let _ = crate::vision::detect::debug_saturation_mask(&frame).save("/tmp/rubic-cam-mask.png");
    eprintln!("rubic: saved /tmp/rubic-cam.png ({w}x{h}) + mask");

    // New multi-face approach: detect individual sticker cells via the lattice.
    let stickers = detect_stickers(&frame);
    eprintln!("rubic: detected {} sticker cells", stickers.len());
    let mut sticker_overlay = frame.clone();
    for &(x0, y0, x1, y1) in &stickers {
        draw_quad(
            &mut sticker_overlay,
            [(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
            image::Rgb([40, 255, 80]),
        );
    }
    let _ = sticker_overlay.save("/tmp/rubic-cam-stickers.png");

    // Full pipeline: fit a face grid and sample its nine colors, drawing each
    // read color at its predicted cell center.
    match read_face_grid_detail(&frame) {
        Some((colors, centers)) => {
            eprintln!("rubic: read face colors {colors:?}");
            let mut overlay = frame.clone();
            for (color, &(cx, cy)) in colors.iter().zip(centers.iter()) {
                imageproc::drawing::draw_filled_circle_mut(
                    &mut overlay,
                    (cx as i32, cy as i32),
                    20,
                    image::Rgb([255, 255, 255]),
                );
                imageproc::drawing::draw_filled_circle_mut(
                    &mut overlay,
                    (cx as i32, cy as i32),
                    15,
                    image::Rgb(*color),
                );
            }
            let _ = overlay.save("/tmp/rubic-cam-read.png");
        }
        None => eprintln!("rubic: no face grid read (see /tmp/rubic-cam.png)"),
    }
}
