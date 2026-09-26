//! Cube color input: painting on the Net or the 3D cube while Editing.
//!
//! The cube being entered lives in the Flow ([`crate::flow::Entry`]); clicks
//! on the Net or on 3D stickers emit `Paint` Actions, and both views render
//! from the Entry.

use bevy::prelude::*;
use rubic_core::{Completion, Face};

use crate::action::Action;
use crate::flow::{Entry, Flow};
use crate::types::{Sticker, StickerMaterials};

/// Palette order (also the number-key order `1..=6`).
pub const PALETTE: [Face; 6] = Face::ALL;

/// Human status line for the Editing HUD.
#[must_use]
pub fn input_status(entry: &Entry) -> String {
    match entry.completion() {
        Completion::Unique(state) => {
            if state.is_solved() {
                "solved".to_string()
            } else {
                "ready - Enter to solve".to_string()
            }
        }
        Completion::NeedMore { known } => format!("{known}/48 painted"),
        Completion::Impossible(err) => format!("impossible - {err}"),
    }
}

/// While Editing, paint the 3D stickers from the Entry (unknown stickers show
/// the neutral "unknown" material). Elsewhere the 3D cube shows the committed
/// cube (see `cube_render::sync_stickers`).
pub fn sync_input_stickers(
    flow: Res<Flow>,
    mats: Res<StickerMaterials>,
    mut stickers: Query<(&Sticker, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let Some(entry) = flow.entry() else {
        return;
    };
    for (sticker, mut material) in &mut stickers {
        let desired = match entry.partial().get(sticker.facelet) {
            Some(face) => &mats.by_face[face.index()],
            None => &mats.unknown,
        };
        if material.0.id() != desired.id() {
            material.0 = desired.clone();
        }
    }
}

/// Observer (an Action adapter): clicking a 3D sticker asks to paint it.
pub fn on_sticker_click(
    click: Trigger<Pointer<Click>>,
    stickers: Query<&Sticker>,
    mut actions: EventWriter<Action>,
) {
    if let Ok(sticker) = stickers.get(click.target()) {
        actions.write(Action::Paint(sticker.facelet));
    }
}
