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
    clippy::type_complexity
)]
#![forbid(unsafe_code)]

mod animation;
mod axis;
mod camera;
mod cli;
mod colors;
mod cube_render;
mod geometry;
mod input;
mod mode;
mod net;
mod paint;
mod solve;
mod types;
mod ui;
mod validation;

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

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "rubic - Rubik's Cube".to_string(),
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
        .add_observer(paint::on_sticker_click)
        .add_systems(
            Startup,
            (
                camera::setup_camera,
                cube_render::setup_cube,
                ui::setup_ui,
                solve::setup_solvers,
                net::setup_net,
                axis::setup_legend,
            ),
        )
        // Always-on: camera, mode switching, net + axis reference, HUD, and the
        // animation driver (which repaints from CubeRes when a turn lands).
        .add_systems(
            Update,
            (
                camera::orbit_camera,
                paint::mode_control,
                net::net_render,
                axis::draw_axes,
                ui::update_status,
                (animation::drive_turns, cube_render::sync_stickers).chain(),
            ),
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
        )
        .run();
}
