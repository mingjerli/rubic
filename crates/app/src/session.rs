//! Shared cube mutations: keep animation, playback, and facelets consistent.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rubic_core::{Facelets, Move};

use crate::solve::SolvePlayer;
use crate::types::{CubeRes, Cubie, TurnQueue};

/// State needed when a user changes the cube outside solution playback.
#[derive(SystemParam)]
pub struct CubeSession<'w, 's> {
    cube: ResMut<'w, CubeRes>,
    queue: ResMut<'w, TurnQueue>,
    player: ResMut<'w, SolvePlayer>,
    cubies: Query<'w, 's, (&'static Cubie, &'static mut Transform)>,
}

impl CubeSession<'_, '_> {
    /// The committed cube, excluding any turn still being animated.
    pub fn facelets(&self) -> Facelets {
        self.cube.0
    }

    /// Discard playback and unfinished turns, restoring the committed view.
    pub fn cancel_playback(&mut self) {
        self.player.player = None;
        self.queue.pending.clear();
        self.queue.active = None;
        for (cubie, mut transform) in &mut self.cubies {
            transform.translation = cubie.home;
            transform.rotation = Quat::IDENTITY;
        }
    }

    /// Replace the cube without leaving moves from the previous state queued.
    pub fn replace(&mut self, facelets: Facelets) {
        self.cancel_playback();
        self.cube.0 = facelets;
    }

    /// Start a manual turn only when idle, invalidating the stored solution.
    pub fn turn(&mut self, mv: Move) {
        if self.queue.is_idle() {
            self.player.player = None;
            self.queue.enqueue(mv);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::picking::pointer::{Location, PointerButton, PointerId};
    use bevy::render::camera::RenderTarget;
    use bevy::window::WindowRef;
    use rubic_core::{Solution, Stage, Step};
    use std::time::Duration;

    use crate::mode::{AppMode, InputStage};
    use crate::paint::InputState;
    use crate::solve::{Solvers, playback::build_player};
    use crate::types::{MainCamera, Sticker};
    use crate::{animation, game, input, paint, play, solve};

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
            .insert_resource(InputState::seeded(&cube))
            .insert_resource(AppMode::Solve)
            .insert_resource(InputStage::Editing)
            .insert_resource(SolvePlayer {
                player: Some(player),
            })
            .init_resource::<TurnQueue>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .add_systems(Update, animation::drive_turns);
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

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
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

    #[test]
    fn reset_cancels_an_in_progress_turn_and_restores_cubies() {
        let mut app = test_app();
        start_turn(&mut app);
        press(&mut app, KeyCode::Backspace);
        app.world_mut()
            .run_system_once(input::manual_input)
            .unwrap();
        assert_eq!(app.world().resource::<CubeRes>().0, Facelets::SOLVED);
        assert_cancelled(&mut app);
    }

    #[test]
    fn shuffle_discards_old_animation_and_solution() {
        let mut app = test_app();
        start_turn(&mut app);
        press(&mut app, KeyCode::KeyG);
        app.world_mut()
            .run_system_once(game::scramble_input)
            .unwrap();
        let cube = app.world().resource::<CubeRes>().0;
        assert!(cube.validate().is_ok());
        assert_ne!(cube, Facelets::SOLVED);
        for i in 0..54 {
            assert_eq!(
                app.world().resource::<InputState>().partial.get(i),
                Some(cube.get(i))
            );
        }
        assert_cancelled(&mut app);
    }

    #[test]
    fn editing_starts_from_committed_cube_and_discards_playback() {
        let mut app = test_app();
        let cube = app.world().resource::<CubeRes>().0;
        start_turn(&mut app);
        press(&mut app, KeyCode::Tab);
        app.world_mut()
            .run_system_once(paint::mode_control)
            .unwrap();
        assert_eq!(*app.world().resource::<AppMode>(), AppMode::Input);
        assert_eq!(*app.world().resource::<InputStage>(), InputStage::Editing);
        for i in 0..54 {
            assert_eq!(
                app.world().resource::<InputState>().partial.get(i),
                Some(cube.get(i))
            );
        }
        assert_cancelled(&mut app);
        assert_eq!(app.world().resource::<CubeRes>().0, cube);
    }

    #[test]
    fn manual_turn_invalidates_solution_only_when_idle() {
        let mut app = test_app();
        press(&mut app, KeyCode::KeyU);
        app.world_mut()
            .run_system_once(input::manual_input)
            .unwrap();
        assert!(app.world().resource::<SolvePlayer>().player.is_none());
        assert_eq!(
            app.world().resource::<TurnQueue>().pending.front(),
            Some(&"U".parse().unwrap())
        );

        let mut app = test_app();
        start_turn(&mut app);
        press(&mut app, KeyCode::KeyU);
        app.world_mut()
            .run_system_once(input::manual_input)
            .unwrap();
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
        press(&mut app, KeyCode::Digit1);
        app.world_mut().run_system_once(solve::solve_input).unwrap();
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
}
