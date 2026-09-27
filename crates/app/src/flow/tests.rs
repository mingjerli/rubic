//! The Flow's transition table, tested without Bevy.

use rubic_core::{Completion, Face, Facelets, Move, PartialFacelets};

use super::*;
use crate::action::{Action, SolverChoice};

const SEED: u64 = 42;

fn ctx() -> StepContext {
    StepContext {
        cube: Facelets::SOLVED.apply(mv("R")),
        seed: SEED,
    }
}

fn step(flow: Flow, action: Action) -> (Flow, Vec<Effect>) {
    flow.step(action, &ctx())
}

fn mv(s: &str) -> Move {
    s.parse().unwrap()
}

fn ready_entry() -> Entry {
    Entry::seeded(&Facelets::SOLVED.apply(mv("F")))
}

/// Every Action, with a representative payload.
fn all_actions() -> Vec<Action> {
    vec![
        Action::Shuffle,
        Action::Manual,
        Action::StartCamera,
        Action::Confirm,
        Action::StartOver,
        Action::Edit,
        Action::Paint(0),
        Action::SelectBrush(Face::R),
        Action::ClearPaint,
        Action::Capture,
        Action::NextFace,
        Action::PrevFace,
        Action::RestartScan,
        Action::Turn(mv("U")),
        Action::ResetCube,
        Action::Solve(SolverChoice::Beginner),
        Action::PlayPause,
        Action::StepForward,
        Action::StepBack,
    ]
}

/// One Flow of each kind (Editing both not-ready and Ready).
fn all_flows() -> Vec<Flow> {
    #[cfg_attr(not(feature = "camera"), allow(unused_mut))]
    let mut flows = vec![
        Flow::Picker,
        Flow::Editing(Entry::blank()),
        Flow::Editing(ready_entry()),
        Flow::Solving,
    ];
    #[cfg(feature = "camera")]
    flows.push(Flow::Scanning(Box::new(scanning_with_face_in_view())));
    flows
}

#[cfg(feature = "camera")]
fn scanning_with_face_in_view() -> crate::scan::Scan {
    let mut scan = crate::scan::Scan::new();
    let face = scan.target().unwrap();
    scan.observe(Some(crate::scan::tests::solved_reading(face)));
    scan
}

fn is_scramble(effects: &[Effect]) -> bool {
    matches!(effects.last(), Some(Effect::ReplaceCube(c)) if *c == scrambled_cube(SEED))
}

// --- Method picker ----------------------------------------------------------

#[test]
fn picker_manual_starts_a_blank_entry() {
    let (flow, effects) = step(Flow::Picker, Action::Manual);
    let entry = flow.entry().expect("Manual goes to Editing");
    assert_eq!(entry.partial().known_count(), 0);
    assert!(effects.is_empty());
}

#[cfg(feature = "camera")]
#[test]
fn picker_start_camera_opens_a_fresh_scan() {
    let (flow, effects) = step(Flow::Picker, Action::StartCamera);
    let scan = flow.scan().expect("StartCamera goes to Scanning");
    assert_eq!(scan.index(), 0);
    assert_eq!(effects, vec![Effect::OpenCamera]);
}

#[test]
fn picker_shuffle_goes_to_solving_with_a_valid_scramble() {
    let (flow, effects) = step(Flow::Picker, Action::Shuffle);
    assert_eq!(flow.kind(), FlowKind::Solving);
    assert_eq!(effects, vec![Effect::ReplaceCube(scrambled_cube(SEED))]);
    let cube = scrambled_cube(SEED);
    assert!(cube.validate().is_ok());
    assert_ne!(cube, Facelets::SOLVED);
}

#[test]
fn picker_ignores_everything_but_the_setup_methods() {
    for action in all_actions() {
        if matches!(
            action,
            Action::Manual | Action::StartCamera | Action::Shuffle
        ) {
            continue;
        }
        let (flow, effects) = step(Flow::Picker, action);
        assert_eq!(flow.kind(), FlowKind::Picker, "{action:?}");
        assert!(effects.is_empty(), "{action:?}");
    }
}

// --- Editing ----------------------------------------------------------------

#[test]
fn editing_paints_with_the_brush_and_clears() {
    let flow = Flow::Editing(Entry::blank());
    let (flow, _) = step(flow, Action::SelectBrush(Face::R));
    let (flow, _) = step(flow, Action::Paint(0));
    let (flow, _) = step(flow, Action::Paint(4)); // a center: locked
    let entry = flow.entry().unwrap();
    assert_eq!(entry.brush(), Face::R);
    assert_eq!(entry.partial().get(0), Some(Face::R));
    assert_eq!(entry.partial().get(4), Some(Face::U));

    let (flow, effects) = step(flow, Action::ClearPaint);
    assert_eq!(flow.entry().unwrap().partial().known_count(), 0);
    assert!(effects.is_empty());
}

#[test]
fn confirm_when_ready_commits_the_cube_and_solves() {
    let (flow, effects) = step(Flow::Editing(ready_entry()), Action::Confirm);
    assert_eq!(flow.kind(), FlowKind::Solving);
    assert_eq!(
        effects,
        vec![Effect::ReplaceCube(Facelets::SOLVED.apply(mv("F")))]
    );
}

#[test]
fn confirm_when_not_ready_stays_editing() {
    let mut contradictory = PartialFacelets::new();
    for i in [0, 2, 6, 8] {
        contradictory = contradictory.set(i, Face::U);
    }
    for i in [9, 11, 15, 17] {
        contradictory = contradictory.set(i, Face::U);
    }
    let impossible = Entry::from_partial(contradictory);
    assert!(matches!(impossible.completion(), Completion::Impossible(_)));

    for entry in [Entry::blank(), impossible] {
        let (flow, effects) = step(Flow::Editing(entry), Action::Confirm);
        assert_eq!(flow.kind(), FlowKind::Editing);
        assert!(effects.is_empty());
    }
}

#[test]
fn editing_start_over_returns_to_a_solved_picker() {
    let (flow, effects) = step(Flow::Editing(ready_entry()), Action::StartOver);
    assert_eq!(flow.kind(), FlowKind::Picker);
    assert_eq!(effects, vec![Effect::ReplaceCube(Facelets::SOLVED)]);
}

#[test]
fn editing_shuffle_goes_to_solving() {
    let (flow, effects) = step(Flow::Editing(Entry::blank()), Action::Shuffle);
    assert_eq!(flow.kind(), FlowKind::Solving);
    assert!(is_scramble(&effects));
}

#[test]
fn readiness_is_computed_once_per_edit() {
    // The cache lives on Entry (see entry.rs); here: Ready survives steps that
    // don't edit, and an edit refreshes it.
    let (flow, _) = step(Flow::Editing(ready_entry()), Action::SelectBrush(Face::B));
    assert!(flow.entry().unwrap().is_ready());
    let (flow, _) = step(flow, Action::ClearPaint);
    assert!(!flow.entry().unwrap().is_ready());
}

// --- Scanning ---------------------------------------------------------------

#[cfg(feature = "camera")]
mod scanning {
    use super::*;
    use crate::scan::Scan;
    use crate::scan::tests::{solid, solved_reading};
    use crate::vision::capture::CAPTURE_ORDER;

    #[test]
    fn capture_with_a_face_in_view_fills_the_live_net() {
        let (flow, effects) = step(
            Flow::Scanning(Box::new(scanning_with_face_in_view())),
            Action::Capture,
        );
        let scan = flow.scan().expect("stays Scanning");
        assert!(scan.current_captured());
        assert_eq!(scan.live().known_count(), 8);
        assert!(effects.is_empty());
    }

    #[test]
    fn capture_with_nothing_in_view_captures_nothing() {
        let (flow, _) = step(Flow::Scanning(Box::default()), Action::Capture);
        assert!(!flow.scan().unwrap().current_captured());
    }

    #[test]
    fn capture_commits_the_reading_shown_as_in_view() {
        let mut scan = Scan::new();
        let face = scan.target().unwrap();
        let other = Face::ALL.into_iter().find(|&f| f != face).unwrap();
        scan.observe(Some(solid(other)));
        scan.observe(Some(solid(face)));
        let (flow, _) = step(Flow::Scanning(Box::new(scan)), Action::Capture);
        assert_eq!(
            flow.scan().unwrap().live().get(face.index() * 9),
            Some(face)
        );
    }

    #[test]
    fn next_prev_and_restart_move_within_the_scan() {
        let (flow, _) = step(
            Flow::Scanning(Box::new(scanning_with_face_in_view())),
            Action::Capture,
        );
        let (flow, effects) = step(flow, Action::NextFace);
        assert_eq!(flow.scan().unwrap().index(), 1);
        assert!(effects.is_empty());
        let (flow, _) = step(flow, Action::PrevFace);
        assert_eq!(flow.scan().unwrap().index(), 0);
        let (flow, _) = step(flow, Action::RestartScan);
        assert!(!flow.scan().unwrap().current_captured());
    }

    #[test]
    fn sixth_next_hands_off_to_editing_and_closes_the_camera() {
        let mut flow = Flow::Scanning(Box::default());
        let mut last = Vec::new();
        for face in CAPTURE_ORDER {
            flow.scan_mut().unwrap().observe(Some(solved_reading(face)));
            (flow, _) = step(flow, Action::Capture);
            (flow, last) = step(flow, Action::NextFace);
        }
        let entry = flow.entry().expect("hands off to Editing");
        assert_eq!(entry.ready_cube(), Some(Facelets::SOLVED));
        assert_eq!(last, vec![Effect::CloseCamera]);
    }

    #[test]
    fn scan_start_over_closes_the_camera_and_returns_to_the_picker() {
        let (flow, effects) = step(Flow::Scanning(Box::default()), Action::StartOver);
        assert_eq!(flow.kind(), FlowKind::Picker);
        assert_eq!(
            effects,
            vec![Effect::CloseCamera, Effect::ReplaceCube(Facelets::SOLVED)]
        );
    }

    #[test]
    fn shuffle_mid_scan_closes_the_camera() {
        let (flow, effects) = step(Flow::Scanning(Box::default()), Action::Shuffle);
        assert_eq!(flow.kind(), FlowKind::Solving);
        assert_eq!(effects[0], Effect::CloseCamera);
        assert!(is_scramble(&effects));
    }

    #[test]
    fn painting_and_turning_do_nothing_mid_scan() {
        for action in [Action::Paint(0), Action::Turn(mv("U")), Action::Confirm] {
            let (flow, effects) = step(Flow::Scanning(Box::default()), action);
            assert_eq!(flow.scan().unwrap().live().known_count(), 0, "{action:?}");
            assert!(effects.is_empty(), "{action:?}");
        }
    }
}

// --- Solving ----------------------------------------------------------------

#[test]
fn solving_turns_and_resets() {
    assert_eq!(
        step(Flow::Solving, Action::Turn(mv("U'"))).1,
        vec![Effect::Turn(mv("U'"))]
    );
    assert_eq!(
        step(Flow::Solving, Action::ResetCube).1,
        vec![Effect::ReplaceCube(Facelets::SOLVED)]
    );
}

#[test]
fn solving_routes_playback_commands() {
    for (action, cmd) in [
        (
            Action::Solve(SolverChoice::Optimal),
            PlaybackCmd::Solve(SolverChoice::Optimal),
        ),
        (Action::PlayPause, PlaybackCmd::PlayPause),
        (Action::StepForward, PlaybackCmd::StepForward),
        (Action::StepBack, PlaybackCmd::StepBack),
    ] {
        let (flow, effects) = step(Flow::Solving, action);
        assert_eq!(flow.kind(), FlowKind::Solving);
        assert_eq!(effects, vec![Effect::Playback(cmd)]);
    }
}

#[test]
fn edit_seeds_editing_from_the_committed_cube() {
    let (flow, effects) = step(Flow::Solving, Action::Edit);
    let entry = flow.entry().expect("Edit goes to Editing");
    assert_eq!(entry.ready_cube(), Some(ctx().cube));
    assert_eq!(effects, vec![Effect::CancelPlayback]);
}

#[test]
fn solving_start_over_returns_to_a_solved_picker() {
    let (flow, effects) = step(Flow::Solving, Action::StartOver);
    assert_eq!(flow.kind(), FlowKind::Picker);
    assert_eq!(effects, vec![Effect::ReplaceCube(Facelets::SOLVED)]);
}

#[test]
fn solving_shuffle_scrambles_again() {
    let (flow, effects) = step(Flow::Solving, Action::Shuffle);
    assert_eq!(flow.kind(), FlowKind::Solving);
    assert!(is_scramble(&effects));
}

// --- Invariants over every state x every Action -----------------------------

#[test]
fn camera_closes_exactly_when_leaving_a_scan() {
    for flow in all_flows() {
        for action in all_actions() {
            let before = flow.kind();
            let (after, effects) = flow.clone().step(action, &ctx());
            let left_scan = before == FlowKind::Scanning && after.kind() != FlowKind::Scanning;
            assert_eq!(
                effects.contains(&Effect::CloseCamera),
                left_scan,
                "{before:?} + {action:?} -> {:?}: {effects:?}",
                after.kind()
            );
        }
    }
}

#[test]
fn camera_opens_only_from_the_picker_on_start_camera() {
    for flow in all_flows() {
        for action in all_actions() {
            let before = flow.kind();
            let (_, effects) = flow.clone().step(action, &ctx());
            if effects.contains(&Effect::OpenCamera) {
                assert_eq!((before, action), (FlowKind::Picker, Action::StartCamera));
            }
        }
    }
}

#[test]
fn entering_solve_always_commits_a_cube() {
    for flow in all_flows() {
        for action in all_actions() {
            let before = flow.kind();
            let (after, effects) = flow.clone().step(action, &ctx());
            if before != FlowKind::Solving && after.kind() == FlowKind::Solving {
                assert!(
                    matches!(effects.last(), Some(Effect::ReplaceCube(_))),
                    "{before:?} + {action:?}: {effects:?}"
                );
            }
        }
    }
}

#[test]
fn unlisted_actions_change_nothing() {
    // (kind, action) pairs that do something; everything else is a no-op.
    let acts = |kind: FlowKind, action: &Action| -> bool {
        use Action as A;
        match kind {
            _ if *action == A::Shuffle => true,
            FlowKind::Picker => matches!(action, A::Manual | A::StartCamera),
            FlowKind::Editing => matches!(
                action,
                A::Paint(_) | A::SelectBrush(_) | A::ClearPaint | A::Confirm | A::StartOver
            ),
            FlowKind::Scanning => matches!(
                action,
                A::Capture | A::NextFace | A::PrevFace | A::RestartScan | A::StartOver
            ),
            FlowKind::Solving => matches!(
                action,
                A::Turn(_)
                    | A::ResetCube
                    | A::Solve(_)
                    | A::PlayPause
                    | A::StepForward
                    | A::StepBack
                    | A::Edit
                    | A::StartOver
            ),
        }
    };
    for flow in all_flows() {
        for action in all_actions() {
            let before = flow.kind();
            if acts(before, &action) {
                continue;
            }
            let (after, effects) = flow.clone().step(action, &ctx());
            assert_eq!(after.kind(), before, "{before:?} + {action:?}");
            assert!(effects.is_empty(), "{before:?} + {action:?}: {effects:?}");
        }
    }
}
