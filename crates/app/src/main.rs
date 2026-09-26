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
//! - [`session`]    cube replacement and manual-turn state management.
//! - [`paint`]      sticker entry and setup-mode transitions.
//! - [`touch`]      on-screen controls for setup and playback.
//! - [`ui`]         on-screen help and status HUD.
//! - [`validation`] cube validity summary for the HUD (pure).

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

mod action;
mod animation;
mod axis;
mod camera;
#[cfg(feature = "camera")]
mod camera_scan;
mod cli;
mod colors;
mod cube_render;
mod flow;
mod game;
mod geometry;
mod layout;
mod net;
mod paint;
mod play;
#[cfg(feature = "camera")]
mod scan;
mod session;
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
use crate::flow::Flow;
use crate::flow::systems::{in_editing, in_solving};
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

    // A CLI-supplied cube drops straight into Editing (review + solve); with no
    // seed we open on the method picker, showing the solved cube as a preview
    // (`facelets` is `SOLVED` here) until the user picks a setup method.
    let seeded = cli.scramble.is_some() || cli.facelets.is_some();
    let flow = if seeded {
        Flow::Editing(flow::Entry::seeded(&facelets))
    } else {
        Flow::Picker
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
    .insert_resource(flow)
    .init_resource::<TurnQueue>()
    .init_resource::<OrbitCamera>()
    .init_resource::<SolvePlayer>()
    .init_resource::<layout::FrameLayout>()
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
    // Input adapters turn keys, taps, clicks and drags into Actions. (The 3D
    // sticker click and drag observers write Actions from the picking step.)
    .add_event::<action::Action>()
    .add_systems(
        Update,
        (
            action::keyboard_actions,
            touch::touch_actions,
            net::net_actions,
        )
            .in_set(action::ActionSources),
    )
    // The one Action consumer: step the Flow and apply its Effects, before the
    // layout, playback and animation read the result.
    .add_systems(
        Update,
        flow::systems::apply_actions
            .after(action::ActionSources)
            .before(layout::update_frame_layout)
            .before(solve::auto_advance),
    )
    // Always-on: camera, net + status HUD, and the animation driver (which
    // repaints from CubeRes when a turn lands).
    .add_systems(
        Update,
        (
            camera::orbit_camera,
            net::net_render,
            ui::update_status,
            (animation::drive_turns, cube_render::sync_stickers).chain(),
        ),
    )
    // Screen layout: recompute the frame layout after this frame's mode
    // changes, then let each element apply its slice of it.
    .add_systems(
        Update,
        (
            layout::update_frame_layout,
            (
                net::toggle_input_ui,
                cube_render::toggle_cube_visibility,
                axis::draw_axes,
                axis::apply_legend_layout,
                ui::apply_desktop_text,
                ui::apply_layout,
                touch::update_touch_controls,
            )
                .after(layout::update_frame_layout),
        ),
    )
    // Solve mode: auto-advance playback.
    .add_systems(
        Update,
        solve::auto_advance
            .before(animation::drive_turns)
            .run_if(in_solving),
    )
    // Editing: the 3D stickers show the cube being entered (elsewhere they show
    // the committed cube), and the Solve button shows readiness.
    .add_systems(
        Update,
        (
            paint::sync_input_stickers.after(cube_render::sync_stickers),
            touch::style_solve_button,
        )
            .run_if(in_editing),
    );

    // Camera cube input (spec 0002), behind the `camera` feature.
    #[cfg(feature = "camera")]
    {
        // The camera starts off; starting a Scan opens it.
        app.insert_non_send_resource(camera_scan::CameraFeed(None))
            .add_systems(
                Startup,
                (
                    camera_scan::setup_camera_hud,
                    camera_scan::setup_camera_preview,
                    camera_scan::setup_camera_buttons,
                ),
            )
            // The frame pump hands each face reading to the Scan before
            // Actions are applied, so a Capture commits the latest reading; the
            // preview, HUD and bar follow the frame layout.
            .add_systems(
                Update,
                (
                    camera_scan::pump_camera.before(flow::systems::apply_actions),
                    camera_scan::camera_button_actions.in_set(action::ActionSources),
                    (
                        camera_scan::apply_camera_layout,
                        camera_scan::update_camera_hud,
                        camera_scan::update_camera_buttons,
                        camera_scan::layout_camera_bar,
                    )
                        .after(layout::update_frame_layout),
                ),
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
