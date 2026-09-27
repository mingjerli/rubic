//! The Flow: where the user is in the app (the method picker, Editing, a Scan,
//! or Solve), and every transition between them.
//!
//! [`Flow::step`] is pure: given an [`Action`] it returns the next Flow and the
//! [`Effect`]s to apply to the rest of the app. The whole transition table is
//! tested without Bevy (see `tests.rs`); [`systems`] is the thin Bevy side
//! that feeds Actions in and applies the Effects. ADR 0001 records why this is
//! a pure state machine rather than Bevy `States`.

use rubic_core::{Facelets, Move};

use crate::action::{Action, SolverChoice};
use crate::game::scrambled_cube;
#[cfg(feature = "camera")]
use crate::scan::Scan;

mod entry;
pub mod systems;
#[cfg(test)]
mod tests;

pub use entry::Entry;

/// Where the user is in the app. Exactly one at a time; each variant holds
/// only the state that exists there.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub enum Flow {
    /// The method picker. The 3D view shows the committed cube.
    #[default]
    Picker,
    /// Painting a cube by hand, or reviewing a finished Scan.
    Editing(Entry),
    /// Capturing the cube's faces with the camera.
    #[cfg(feature = "camera")]
    Scanning(Box<Scan>),
    /// Playing the committed cube, or following a Solution.
    Solving,
}

/// Which [`Flow`] the user is in, without its data (for layout and keymaps).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlowKind {
    Picker,
    Editing,
    #[cfg_attr(not(feature = "camera"), allow(dead_code))]
    Scanning,
    Solving,
}

/// What a transition asks of the rest of the app. A closed set: a new kind of
/// side effect is a design change, so review it here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Commit a new cube, dropping any playback and animating turn.
    ReplaceCube(Facelets),
    /// Drop playback and any animating turn, keeping the committed cube.
    CancelPlayback,
    /// Turn a face (only when no turn is animating).
    Turn(Move),
    Playback(PlaybackCmd),
    #[cfg_attr(not(feature = "camera"), allow(dead_code))]
    OpenCamera,
    /// Release the camera device.
    #[cfg_attr(not(feature = "camera"), allow(dead_code))]
    CloseCamera,
}

/// A Playback command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackCmd {
    Solve(SolverChoice),
    PlayPause,
    StepForward,
    StepBack,
}

/// What a step may read from outside the Flow.
#[derive(Clone, Copy, Debug)]
pub struct StepContext {
    /// The committed cube.
    pub cube: Facelets,
    /// Seed for a Shuffle.
    pub seed: u64,
}

impl Flow {
    #[must_use]
    pub fn kind(&self) -> FlowKind {
        match self {
            Flow::Picker => FlowKind::Picker,
            Flow::Editing(_) => FlowKind::Editing,
            #[cfg(feature = "camera")]
            Flow::Scanning(_) => FlowKind::Scanning,
            Flow::Solving => FlowKind::Solving,
        }
    }

    /// The cube being entered, while Editing.
    #[must_use]
    pub fn entry(&self) -> Option<&Entry> {
        match self {
            Flow::Editing(entry) => Some(entry),
            _ => None,
        }
    }

    /// The Scan in progress, while Scanning.
    #[cfg(feature = "camera")]
    #[must_use]
    pub fn scan(&self) -> Option<&Scan> {
        match self {
            Flow::Scanning(scan) => Some(scan.as_ref()),
            _ => None,
        }
    }

    #[cfg(feature = "camera")]
    pub fn scan_mut(&mut self) -> Option<&mut Scan> {
        match self {
            Flow::Scanning(scan) => Some(scan.as_mut()),
            _ => None,
        }
    }

    /// Apply `action`: the next Flow and the Effects the transition asks for.
    /// An Action that doesn't apply where the user is changes nothing.
    #[must_use]
    pub fn step(self, action: Action, ctx: &StepContext) -> (Flow, Vec<Effect>) {
        // Shuffle works from anywhere.
        if action == Action::Shuffle {
            let mut effects = self.exit_effects();
            effects.push(Effect::ReplaceCube(scrambled_cube(ctx.seed)));
            return (Flow::Solving, effects);
        }
        match self {
            Flow::Picker => picker(action),
            Flow::Editing(entry) => editing(entry, action),
            #[cfg(feature = "camera")]
            Flow::Scanning(scan) => scanning(scan, action),
            Flow::Solving => solving(action, ctx),
        }
    }

    /// Effects of leaving this Flow for another: a Scan releases the camera.
    fn exit_effects(&self) -> Vec<Effect> {
        match self {
            #[cfg(feature = "camera")]
            Flow::Scanning(_) => vec![Effect::CloseCamera],
            _ => Vec::new(),
        }
    }
}

/// Back to the method picker, showing a solved cube.
fn start_over(mut effects: Vec<Effect>) -> (Flow, Vec<Effect>) {
    effects.push(Effect::ReplaceCube(Facelets::SOLVED));
    (Flow::Picker, effects)
}

fn picker(action: Action) -> (Flow, Vec<Effect>) {
    match action {
        Action::Manual => (Flow::Editing(Entry::blank()), Vec::new()),
        #[cfg(feature = "camera")]
        Action::StartCamera => (Flow::Scanning(Box::default()), vec![Effect::OpenCamera]),
        _ => (Flow::Picker, Vec::new()),
    }
}

fn editing(mut entry: Entry, action: Action) -> (Flow, Vec<Effect>) {
    match action {
        Action::Paint(facelet) => entry.paint(facelet),
        Action::SelectBrush(face) => entry.select(face),
        Action::ClearPaint => entry.clear(),
        Action::Confirm => {
            if let Some(cube) = entry.ready_cube() {
                return (Flow::Solving, vec![Effect::ReplaceCube(cube)]);
            }
        }
        Action::StartOver => return start_over(Vec::new()),
        _ => {}
    }
    (Flow::Editing(entry), Vec::new())
}

#[cfg(feature = "camera")]
fn scanning(mut scan: Box<Scan>, action: Action) -> (Flow, Vec<Effect>) {
    match action {
        Action::Capture => {
            scan.capture();
        }
        Action::NextFace => {
            if let Some(scanned) = scan.next_face() {
                return (
                    Flow::Editing(Entry::from_partial(scanned)),
                    vec![Effect::CloseCamera],
                );
            }
        }
        Action::PrevFace => scan.prev_face(),
        Action::RestartScan => scan.restart(),
        Action::StartOver => return start_over(vec![Effect::CloseCamera]),
        _ => {}
    }
    (Flow::Scanning(scan), Vec::new())
}

fn solving(action: Action, ctx: &StepContext) -> (Flow, Vec<Effect>) {
    let effect = match action {
        Action::Turn(mv) => Effect::Turn(mv),
        Action::ResetCube => Effect::ReplaceCube(Facelets::SOLVED),
        Action::Solve(choice) => Effect::Playback(PlaybackCmd::Solve(choice)),
        Action::PlayPause => Effect::Playback(PlaybackCmd::PlayPause),
        Action::StepForward => Effect::Playback(PlaybackCmd::StepForward),
        Action::StepBack => Effect::Playback(PlaybackCmd::StepBack),
        Action::Edit => {
            return (
                Flow::Editing(Entry::seeded(&ctx.cube)),
                vec![Effect::CancelPlayback],
            );
        }
        Action::StartOver => return start_over(Vec::new()),
        _ => return (Flow::Solving, Vec::new()),
    };
    (Flow::Solving, vec![effect])
}
