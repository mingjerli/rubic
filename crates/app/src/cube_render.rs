//! Building and syncing the 3D cube.
//!
//! `setup_cube` spawns 27 dark cubie bodies plus one colored sticker quad per
//! exposed facelet (each tagged with its facelet index and parented to its
//! cubie so animations move them together). `sync_stickers` repaints those
//! quads whenever [`CubeRes`] changes by swapping to a precomputed per-color
//! material - no asset mutation, no per-frame allocation.
//!
//! Called by `main.rs` (startup + update schedules).

use bevy::prelude::*;
use rubic_core::Face;

use crate::colors::{body_rgb, sticker_rgb};
use crate::geometry::{all_cubies, all_stickers};
use crate::types::{CubeRes, Cubie, Sticker, StickerMaterials};

/// World distance between adjacent cubie centers.
const SPACING: f32 = 1.0;
/// Edge length of a cubie body (slightly under `SPACING` for visible gaps).
const CUBIE_SIZE: f32 = 0.96;
/// Edge length of a sticker quad.
const STICKER_SIZE: f32 = 0.84;
/// How far a sticker floats off the cubie center along its normal.
const STICKER_OFFSET: f32 = 0.5 * CUBIE_SIZE + 0.006;

/// Convert an sRGB triple into a matte `StandardMaterial`.
fn matte(rgb: [f32; 3]) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(rgb[0], rgb[1], rgb[2]),
        perceptual_roughness: 0.6,
        reflectance: 0.08,
        ..default()
    }
}

/// Spawn cubie bodies, sticker quads, materials, and lighting.
pub fn setup_cube(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let by_face: [Handle<StandardMaterial>; 6] =
        Face::ALL.map(|face| materials.add(matte(sticker_rgb(face))));
    let body_material = materials.add(matte(body_rgb()));
    let body_mesh = meshes.add(Cuboid::new(CUBIE_SIZE, CUBIE_SIZE, CUBIE_SIZE));
    let sticker_mesh = meshes.add(Rectangle::new(STICKER_SIZE, STICKER_SIZE));

    let stickers = all_stickers();

    for cell in all_cubies() {
        let home = Vec3::new(
            cell[0] as f32 * SPACING,
            cell[1] as f32 * SPACING,
            cell[2] as f32 * SPACING,
        );

        commands
            .spawn((
                Mesh3d(body_mesh.clone()),
                MeshMaterial3d(body_material.clone()),
                Transform::from_translation(home),
                Cubie { cell, home },
            ))
            .with_children(|parent| {
                for spec in stickers.iter().filter(|s| s.cubie == cell) {
                    let normal = Vec3::new(
                        spec.normal[0] as f32,
                        spec.normal[1] as f32,
                        spec.normal[2] as f32,
                    );
                    let face = Face::ALL[spec.facelet / 9];
                    let local = Transform {
                        translation: normal * STICKER_OFFSET,
                        rotation: Quat::from_rotation_arc(Vec3::Z, normal),
                        ..default()
                    };
                    parent.spawn((
                        Mesh3d(sticker_mesh.clone()),
                        MeshMaterial3d(by_face[face.index()].clone()),
                        local,
                        Sticker {
                            facelet: spec.facelet,
                        },
                    ));
                }
            });
    }

    commands.insert_resource(StickerMaterials { by_face });

    // Lighting: a key directional light plus soft ambient fill.
    commands.spawn((
        DirectionalLight {
            illuminance: 6000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(6.0, 10.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 350.0,
        ..default()
    });
}

/// Repaint sticker quads to match the current facelets whenever they change.
pub fn sync_stickers(
    cube: Res<CubeRes>,
    palette: Res<StickerMaterials>,
    mut stickers: Query<(&Sticker, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if !cube.is_changed() {
        return;
    }
    for (sticker, mut material) in &mut stickers {
        let face = cube.0.get(sticker.facelet);
        let desired = &palette.by_face[face.index()];
        if material.0.id() != desired.id() {
            material.0 = desired.clone();
        }
    }
}
