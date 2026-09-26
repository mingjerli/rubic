//! Headless-App tests: Actions through the Flow into the committed cube,
//! playback, animation and camera.

use super::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::picking::pointer::{Location, PointerButton, PointerId};
use bevy::render::camera::RenderTarget;
use bevy::window::WindowRef;
use rubic_core::{Solution, Stage, Step};
use std::time::Duration;

use crate::action::{Action, SolverChoice};
use crate::flow::systems::apply_actions;
use crate::flow::{Flow, FlowKind};
use crate::solve::{Solvers, playback::build_player};
use crate::touch::{self, TouchControl};
use crate::types::{MainCamera, Sticker};
use crate::{animation, play, solve};

/// Solving `R` applied to a solved cube, with a one-move solution playing.
fn test_app() -> App {
    let mv: Move = "R".parse().unwrap();
    let cube = Facelets::SOLVED.apply(mv);
    let mut player = build_player(
        &Solution {
            steps: vec![Step {
                moves: vec![mv.inverse()],
                stage: Stage::Optimal,
                note: String::new(),
            }],
        },
        "Test",
    );
    player.playing = true;
    let mut app = App::new();
    app.insert_resource(CubeRes(cube))
        .insert_resource(Flow::Solving)
        .insert_resource(SolvePlayer {
            player: Some(player),
        })
        .init_resource::<TurnQueue>()
        .add_event::<Action>()
        .init_resource::<Time>()
        .add_systems(Update, animation::drive_turns);
    #[cfg(feature = "camera")]
    app.insert_non_send_resource(crate::camera_scan::CameraFeed(None));
    app.world_mut().spawn((
        Cubie {
            cell: [1, 1, 1],
            home: Vec3::ONE,
        },
        Transform::from_translation(Vec3::ONE),
    ));
    app
}

fn start_turn(app: &mut App) {
    app.world_mut()
        .run_system_once(solve::auto_advance)
        .unwrap();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(40));
    app.update();
    assert!(app.world().resource::<TurnQueue>().active.is_some());
    let world = app.world_mut();
    let mut query = world.query::<&Transform>();
    assert_ne!(query.single(world).unwrap().rotation, Quat::IDENTITY);
}

/// Send `action` and apply it.
fn act(app: &mut App, action: Action) {
    app.world_mut().send_event(action);
    app.world_mut().run_system_once(apply_actions).unwrap();
}

fn flow_kind(app: &App) -> FlowKind {
    app.world().resource::<Flow>().kind()
}

fn assert_cancelled(app: &mut App) {
    // Let the animation driver run again: cancelled moves must not land.
    let expected = app.world().resource::<CubeRes>().0;
    app.update();
    assert_eq!(app.world().resource::<CubeRes>().0, expected);
    assert!(app.world().resource::<TurnQueue>().is_idle());
    assert!(app.world().resource::<SolvePlayer>().player.is_none());
    let world = app.world_mut();
    let mut query = world.query::<(&Cubie, &Transform)>();
    for (cubie, transform) in query.iter(world) {
        assert_eq!(transform.translation, cubie.home);
        assert_eq!(transform.rotation, Quat::IDENTITY);
    }
}

fn set_interaction(app: &mut App, button: Entity, interaction: Interaction) {
    *app.world_mut().get_mut::<Interaction>(button).unwrap() = interaction;
}

#[test]
fn shuffle_button_keeps_working_after_the_first_tap() {
    let mut app = test_app();
    app.add_systems(Update, (touch::touch_actions, apply_actions).chain());
    let button = app
        .world_mut()
        .spawn((TouchControl::Shuffle, Interaction::Pressed))
        .id();
    app.update();
    let first = app.world().resource::<CubeRes>().0;
    assert_ne!(first, Facelets::SOLVED.apply("R".parse().unwrap()));

    set_interaction(&mut app, button, Interaction::None);
    app.update();
    set_interaction(&mut app, button, Interaction::Pressed);
    app.update();
    let second = app.world().resource::<CubeRes>().0;
    assert_ne!(second, first, "second tap must shuffle again");
    assert_eq!(flow_kind(&app), FlowKind::Solving);
}

#[test]
fn a_key_press_reaches_the_flow_in_the_same_frame() {
    let mut app = test_app();
    app.init_resource::<ButtonInput<KeyCode>>().add_systems(
        Update,
        (crate::action::keyboard_actions, apply_actions).chain(),
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Tab);
    app.update();
    assert_eq!(flow_kind(&app), FlowKind::Editing);
}

#[test]
fn an_action_for_another_flow_is_ignored() {
    let mut app = test_app();
    let cube = app.world().resource::<CubeRes>().0;
    // A Scan's Capture, or painting, means nothing while solving.
    act(&mut app, Action::Capture);
    act(&mut app, Action::Paint(0));
    assert_eq!(flow_kind(&app), FlowKind::Solving);
    assert_eq!(app.world().resource::<CubeRes>().0, cube);
    assert!(app.world().resource::<SolvePlayer>().player.is_some());
}

#[test]
fn reset_cancels_an_in_progress_turn_and_restores_cubies() {
    let mut app = test_app();
    start_turn(&mut app);
    act(&mut app, Action::ResetCube);
    assert_eq!(app.world().resource::<CubeRes>().0, Facelets::SOLVED);
    assert_cancelled(&mut app);
}

#[test]
fn shuffle_discards_old_animation_and_solution() {
    let mut app = test_app();
    start_turn(&mut app);
    act(&mut app, Action::Shuffle);
    let cube = app.world().resource::<CubeRes>().0;
    assert!(cube.validate().is_ok());
    assert_ne!(cube, Facelets::SOLVED);
    assert_cancelled(&mut app);
}

#[test]
fn editing_starts_from_committed_cube_and_discards_playback() {
    let mut app = test_app();
    let cube = app.world().resource::<CubeRes>().0;
    start_turn(&mut app);
    act(&mut app, Action::Edit);
    let entry = app.world().resource::<Flow>().entry().cloned();
    assert_eq!(
        entry.expect("Edit goes to Editing").ready_cube(),
        Some(cube)
    );
    assert_cancelled(&mut app);
    assert_eq!(app.world().resource::<CubeRes>().0, cube);
}

#[test]
fn manual_turn_invalidates_solution_only_when_idle() {
    let mut app = test_app();
    act(&mut app, Action::Turn("U".parse().unwrap()));
    assert!(app.world().resource::<SolvePlayer>().player.is_none());
    assert_eq!(
        app.world().resource::<TurnQueue>().pending.front(),
        Some(&"U".parse().unwrap())
    );

    let mut app = test_app();
    start_turn(&mut app);
    act(&mut app, Action::Turn("U".parse().unwrap()));
    assert!(app.world().resource::<SolvePlayer>().player.is_some());
    let queue = app.world().resource::<TurnQueue>();
    assert!(queue.pending.is_empty());
    assert_eq!(queue.active.unwrap().mv, "R'".parse().unwrap());
}

#[test]
fn drag_turn_invalidates_solution() {
    let mut app = test_app();
    app.init_resource::<play::OrbitSuppressed>()
        .add_observer(play::on_drag_end);
    let window = app.world_mut().spawn_empty().id();
    let sticker = app.world_mut().spawn(Sticker { facelet: 18 }).id();
    app.world_mut()
        .spawn((MainCamera, GlobalTransform::IDENTITY));
    app.world_mut().trigger_targets(
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: RenderTarget::Window(WindowRef::Entity(window))
                    .normalize(None)
                    .unwrap(),
                position: Vec2::ZERO,
            },
            sticker,
            DragEnd {
                button: PointerButton::Primary,
                distance: Vec2::new(40.0, 0.0),
            },
        ),
        sticker,
    );
    app.world_mut().run_system_once(apply_actions).unwrap();
    assert!(app.world().resource::<SolvePlayer>().player.is_none());
    assert_eq!(app.world().resource::<TurnQueue>().pending.len(), 1);
}

#[test]
fn solving_while_a_turn_is_pending_keeps_existing_playback() {
    let mut app = test_app();
    app.init_resource::<Solvers>();
    app.world_mut()
        .resource_mut::<TurnQueue>()
        .enqueue("U".parse().unwrap());
    act(&mut app, Action::Solve(SolverChoice::Beginner));
    let player = app
        .world()
        .resource::<SolvePlayer>()
        .player
        .as_ref()
        .unwrap();
    assert_eq!(player.solver_name, "Test");
    assert!(player.playing);
    assert_eq!(player.cursor, 0);
}

#[cfg(feature = "camera")]
mod camera {
    use super::*;
    use crate::camera_scan::CameraFeed;
    use crate::scan::Scan;
    use crate::vision::source::ReplaySource;

    fn camera_open(app: &App) -> bool {
        app.world().non_send_resource::<CameraFeed>().0.is_some()
    }

    #[test]
    fn shuffle_mid_scan_closes_the_camera() {
        let mut app = test_app();
        *app.world_mut().resource_mut::<Flow>() = Flow::Scanning(Scan::new());
        app.world_mut().non_send_resource_mut::<CameraFeed>().0 =
            Some(Box::new(ReplaySource::new(Vec::new())));
        app.init_resource::<ButtonInput<KeyCode>>().add_systems(
            Update,
            (crate::action::keyboard_actions, apply_actions).chain(),
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        assert_eq!(flow_kind(&app), FlowKind::Solving);
        assert!(!camera_open(&app), "leaving a Scan must release the camera");
    }

    #[test]
    fn start_camera_without_a_camera_stays_on_the_picker() {
        // Test builds have no camera backend, so opening always fails.
        let mut app = test_app();
        *app.world_mut().resource_mut::<Flow>() = Flow::Picker;
        act(&mut app, Action::StartCamera);
        assert_eq!(flow_kind(&app), FlowKind::Picker);
        assert!(!camera_open(&app));
        assert_eq!(app.world().resource::<CubeRes>().0, Facelets::SOLVED);
    }
}
