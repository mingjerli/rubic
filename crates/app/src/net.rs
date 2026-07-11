//! The 2D unfolded-net input panel and color palette.
//!
//! The net is an always-visible, labelled reference (top-right) and, in input
//! mode, a paint surface: clicking a cell paints it with the selected color.
//! It reads/writes the shared [`InputState`]. The palette (bottom-right) selects
//! the brush color. Pure layout math (`face_grid`, `cell_facelet`) is tested.

use bevy::prelude::*;
use rubic_core::Face;

use crate::colors::sticker_rgb;
use crate::mode::{AppMode, InputStage};
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

/// Overall net dimensions (4 faces wide, 3 tall) and palette width, for the
/// responsive layout to position/center them.
pub const NET_W: f32 = 4.0 * STRIDE;
pub const NET_H: f32 = 3.0 * STRIDE;

/// Marker for the net container (repositioned per screen size).
#[derive(Component)]
pub struct NetRoot;

/// Marker for the palette container (repositioned per screen size).
#[derive(Component)]
pub struct PaletteRoot;

fn srgb(face: Face) -> Color {
    let c = sticker_rgb(face);
    Color::srgb(c[0], c[1], c[2])
}

const UNKNOWN: Color = Color::srgb(0.22, 0.23, 0.26);

/// Spawn the net grid (top-right) and the color palette (below it).
pub fn setup_net(mut commands: Commands) {
    // Net container.
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                right: Val::Px(8.0),
                width: Val::Px(NET_W),
                height: Val::Px(NET_H),
                ..default()
            },
            NetRoot,
        ))
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

    // Color palette, tucked into the net's empty top-right corner (the cross
    // leaves cols 2-3 of the top row blank) as a 3-wide x 2-tall grid, so it
    // reuses dead space instead of taking a row below. As a child of the net it
    // tracks the net's position on every screen size.
    let sw = 30.0;
    let gap = 6.0;
    let pal_w = 3.0 * sw + 2.0 * gap;
    commands.entity(root).with_children(|parent| {
        parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(4.0),
                    left: Val::Px(NET_W - pal_w - 6.0),
                    width: Val::Px(pal_w),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(gap),
                    row_gap: Val::Px(gap),
                    ..default()
                },
                PaletteRoot,
            ))
            .with_children(|pal| {
                for face in PALETTE {
                    pal.spawn((
                        Button,
                        Node {
                            width: Val::Px(sw),
                            height: Val::Px(sw),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BorderColor(Color::srgb(0.1, 0.1, 0.12)),
                        BackgroundColor(srgb(face)),
                        PaletteSwatch { face },
                    ));
                }
            });
    });
}

/// Whether the 2D net is shown. Hidden while solving (the 3D cube is the only
/// view) and on the Input method-picker (a solved 3D preview stands in); shown
/// while editing a cube by hand and while the camera scan fills it in.
#[must_use]
pub fn net_visible(mode: AppMode, stage: InputStage) -> bool {
    match mode {
        AppMode::Solve => false,
        AppMode::Camera => true,
        AppMode::Input => stage == InputStage::Editing,
    }
}

/// Whether the color palette is shown: only while painting by hand (Input mode,
/// editing). Hidden on the method-picker, during camera scan, and while solving.
#[must_use]
pub fn palette_visible(mode: AppMode, stage: InputStage) -> bool {
    mode == AppMode::Input && stage == InputStage::Editing
}

/// Show the net + palette only when they're useful (see [`net_visible`] /
/// [`palette_visible`]).
#[allow(clippy::type_complexity)]
pub fn toggle_input_ui(
    mode: Res<AppMode>,
    stage: Res<InputStage>,
    mut net: Query<&mut Visibility, (With<NetRoot>, Without<PaletteRoot>)>,
    mut palette: Query<&mut Visibility, (With<PaletteRoot>, Without<NetRoot>)>,
) {
    let net_want = vis(net_visible(*mode, *stage));
    for mut v in &mut net {
        if *v != net_want {
            *v = net_want;
        }
    }
    let palette_want = vis(palette_visible(*mode, *stage));
    for mut v in &mut palette {
        if *v != palette_want {
            *v = palette_want;
        }
    }
}

/// Map a "should show" flag to a Bevy visibility.
fn vis(show: bool) -> Visibility {
    if show {
        Visibility::Visible
    } else {
        Visibility::Hidden
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

    #[test]
    fn net_shows_only_while_editing_or_scanning() {
        use InputStage::{ChooseMethod, Editing};
        // Method picker: hidden (a solved 3D preview stands in).
        assert!(!net_visible(AppMode::Input, ChooseMethod));
        // Editing by hand / reviewing a scan: shown.
        assert!(net_visible(AppMode::Input, Editing));
        // Camera scan fills the net live: shown regardless of stage.
        assert!(net_visible(AppMode::Camera, ChooseMethod));
        // Solving: only the 3D cube.
        assert!(!net_visible(AppMode::Solve, Editing));
    }

    #[test]
    fn palette_shows_only_while_painting() {
        use InputStage::{ChooseMethod, Editing};
        assert!(palette_visible(AppMode::Input, Editing));
        assert!(!palette_visible(AppMode::Input, ChooseMethod));
        assert!(!palette_visible(AppMode::Camera, Editing));
        assert!(!palette_visible(AppMode::Solve, Editing));
    }
}
