//! Orientation reference: a colored axis triad drawn beside the cube plus a
//! move legend.
//!
//! The cube lives in world space and the camera orbits it, so world-space axis
//! arrows drawn from the origin turn by exactly the same angle as the cube. Each
//! arrow is colored with its face's sticker color and points out that face:
//! +X=R, -X=L, +Y=U, -Y=D, +Z=F, -Z=B. The legend (text) spells out how each
//! move rotates about these axes.

use bevy::prelude::*;
use rubic_core::Face;

use crate::colors::sticker_rgb;

/// The face an axis end points out of. `axis`: 0=X, 1=Y, 2=Z.
#[must_use]
pub fn axis_face(axis: usize, positive: bool) -> Face {
    match (axis, positive) {
        (0, true) => Face::R,
        (0, false) => Face::L,
        (1, true) => Face::U,
        (1, false) => Face::D,
        (2, true) => Face::F,
        _ => Face::B,
    }
}

/// Unit direction for an axis end.
fn axis_dir(axis: usize, positive: bool) -> Vec3 {
    let mut v = Vec3::ZERO;
    v[axis] = if positive { 1.0 } else { -1.0 };
    v
}

const AXIS_LEN: f32 = 2.9;

/// Draw the six face-colored axis arrows each frame (they orbit with the cube).
pub fn draw_axes(mut gizmos: Gizmos) {
    for axis in 0..3 {
        for positive in [true, false] {
            let dir = axis_dir(axis, positive);
            let rgb = sticker_rgb(axis_face(axis, positive));
            gizmos.arrow(
                Vec3::ZERO,
                dir * AXIS_LEN,
                Color::srgb(rgb[0], rgb[1], rgb[2]),
            );
        }
    }
}

const LEGEND: &str = "\
Orientation (axes turn with the cube):
  +X = R   -X = L    (red / orange arrow)
  +Y = U   -Y = D    (white / yellow arrow)
  +Z = F   -Z = B    (green / blue arrow)
A face key turns 90 degrees about that face's axis.
  Shift = reverse direction (')    2 = 180 degrees";

/// Marker for the legend text.
#[derive(Component)]
pub struct LegendText;

/// Spawn the static move/orientation legend (bottom-right).
pub fn setup_legend(mut commands: Commands) {
    commands.spawn((
        Text::new(LEGEND),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.86, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            right: Val::Px(8.0),
            ..default()
        },
        LegendText,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_faces_cover_all_six() {
        let mut seen = Vec::new();
        for axis in 0..3 {
            for positive in [true, false] {
                seen.push(axis_face(axis, positive));
            }
        }
        for face in Face::ALL {
            assert!(seen.contains(&face), "missing {}", face.to_char());
        }
    }

    #[test]
    fn axis_directions_are_unit_axes() {
        for axis in 0..3 {
            let p = axis_dir(axis, true);
            let n = axis_dir(axis, false);
            assert_eq!(p, -n);
            assert!((p.length() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn positive_axes_match_r_u_f() {
        assert_eq!(axis_face(0, true), Face::R);
        assert_eq!(axis_face(1, true), Face::U);
        assert_eq!(axis_face(2, true), Face::F);
    }
}
