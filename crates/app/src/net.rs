//! The 2D unfolded-net input panel and color palette.
//!
//! The net is an always-visible, labelled reference (top-right) and, in input
//! mode, a paint surface: clicking a cell paints it with the selected color.
//! It reads/writes the shared [`InputState`]. The palette (bottom-right) selects
//! the brush color. Pure layout math (`face_grid`, `cell_facelet`) is tested.

use bevy::prelude::*;
use rubic_core::Face;

use crate::colors::sticker_rgb;
use crate::paint::{InputState, PALETTE};

/// Grid position `(row, col)` of a face in the unfolded cross (3 rows x 4 cols).
#[must_use]
pub fn face_grid(face: Face) -> (usize, usize) {
    match face {
        Face::U => (0, 1),
        Face::L => (1, 0),
        Face::F => (1, 1),
        Face::R => (1, 2),
        Face::B => (1, 3),
        Face::D => (2, 1),
    }
}

/// Facelet index for cell `(r, c)` within `face`.
#[must_use]
pub fn cell_facelet(face: Face, r: usize, c: usize) -> usize {
    face.index() * 9 + r * 3 + c
}

/// A clickable net cell, tagged with the facelet it edits.
#[derive(Component, Clone, Copy)]
pub struct NetCell {
    /// Facelet index (`0..54`).
    pub facelet: usize,
}

/// A palette swatch that selects a paint color.
#[derive(Component, Clone, Copy)]
pub struct PaletteSwatch {
    /// The color this swatch selects.
    pub face: Face,
}

const CELL: f32 = 22.0;
const GAP: f32 = 2.0;
const FACE_GAP: f32 = 6.0;
const BLOCK: f32 = 3.0 * CELL + 2.0 * GAP; // one face
const STRIDE: f32 = BLOCK + FACE_GAP; // face-to-face

fn srgb(face: Face) -> Color {
    let c = sticker_rgb(face);
    Color::srgb(c[0], c[1], c[2])
}

const UNKNOWN: Color = Color::srgb(0.22, 0.23, 0.26);

/// Spawn the net grid (top-right) and the color palette (below it).
pub fn setup_net(mut commands: Commands) {
    // Net container.
    let root = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            right: Val::Px(8.0),
            width: Val::Px(4.0 * STRIDE),
            height: Val::Px(3.0 * STRIDE),
            ..default()
        })
        .id();

    for face in Face::ALL {
        let (gr, gc) = face_grid(face);
        for r in 0..3 {
            for c in 0..3 {
                let facelet = cell_facelet(face, r, c);
                let x = gc as f32 * STRIDE + c as f32 * (CELL + GAP);
                let y = gr as f32 * STRIDE + r as f32 * (CELL + GAP);
                commands.entity(root).with_children(|parent| {
                    parent.spawn((
                        Button,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x),
                            top: Val::Px(y),
                            width: Val::Px(CELL),
                            height: Val::Px(CELL),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor(Color::srgb(0.1, 0.1, 0.12)),
                        BackgroundColor(UNKNOWN),
                        NetCell { facelet },
                    ));
                });
            }
        }
    }

    // Palette row, below the net.
    let palette = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0 + 3.0 * STRIDE + 10.0),
            right: Val::Px(8.0),
            column_gap: Val::Px(6.0),
            ..default()
        })
        .id();
    for face in PALETTE {
        commands.entity(palette).with_children(|parent| {
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor(Color::srgb(0.1, 0.1, 0.12)),
                BackgroundColor(srgb(face)),
                PaletteSwatch { face },
            ));
        });
    }
}

/// Paint a net cell when clicked (input mode).
pub fn net_click(
    cells: Query<(&Interaction, &NetCell), Changed<Interaction>>,
    mut input: ResMut<InputState>,
) {
    for (interaction, cell) in &cells {
        if *interaction == Interaction::Pressed {
            input.paint(cell.facelet);
        }
    }
}

/// Select a color when its swatch is clicked (input mode).
pub fn palette_click(
    swatches: Query<(&Interaction, &PaletteSwatch), Changed<Interaction>>,
    mut input: ResMut<InputState>,
) {
    for (interaction, swatch) in &swatches {
        if *interaction == Interaction::Pressed {
            input.select(swatch.face);
        }
    }
}

/// Repaint net cells from the partial state and highlight the selected swatch.
pub fn net_render(
    input: Res<InputState>,
    mut cells: Query<(&NetCell, &mut BackgroundColor), Without<PaletteSwatch>>,
    mut swatches: Query<(&PaletteSwatch, &mut BorderColor)>,
) {
    for (cell, mut bg) in &mut cells {
        bg.0 = match input.partial.get(cell.facelet) {
            Some(face) => srgb(face),
            None => UNKNOWN,
        };
    }
    for (swatch, mut border) in &mut swatches {
        border.0 = if swatch.face == input.brush {
            Color::WHITE
        } else {
            Color::srgb(0.1, 0.1, 0.12)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_face_has_a_distinct_grid_slot() {
        let mut seen = Vec::new();
        for face in Face::ALL {
            let slot = face_grid(face);
            assert!(
                !seen.contains(&slot),
                "duplicate slot for {}",
                face.to_char()
            );
            seen.push(slot);
        }
    }

    #[test]
    fn cell_facelet_round_trips_all_54() {
        let mut seen = [false; 54];
        for face in Face::ALL {
            for r in 0..3 {
                for c in 0..3 {
                    let i = cell_facelet(face, r, c);
                    assert!(i < 54);
                    assert!(!seen[i], "facelet {i} produced twice");
                    seen[i] = true;
                }
            }
        }
        assert!(seen.iter().all(|&b| b));
    }

    #[test]
    fn center_cells_are_the_face_centers() {
        for face in Face::ALL {
            assert_eq!(cell_facelet(face, 1, 1), face.index() * 9 + 4);
        }
    }
}
