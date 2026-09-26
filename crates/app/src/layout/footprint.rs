//! Estimated on-screen footprints of a [`FrameLayout`], for the overlap tests.
//!
//! Text and buttons are sized with the default font's metrics (see
//! [`super::text`]). The 3D cube is projected as its bounding sphere under the
//! default orbit angles, which slightly over-estimates its silhouette. Status
//! and HUD text use the longest content each state can show.

use rubic_core::Face;

use super::text::{RowAlign, button_size, rows_height, text_size, wrap_rows};
use super::*;
use crate::touch::TouchControl;

/// Size of a top-bar button.
fn top_button_size(control: TouchControl) -> Vec2 {
    button_size(
        control.label(),
        control.hint().map(|h| (h, TOP_HINT_FONT)),
        TOP_BUTTON_FONT,
        TOP_BUTTON_PAD,
        BUTTON_BORDER,
    )
}

/// Bevy's default perspective field of view (vertical).
const FOV_Y: f32 = std::f32::consts::FRAC_PI_4;
/// Bounding-sphere radius of the cube: cubie centers at +-1, half-size 0.48.
const CUBE_SPHERE: f32 = 1.48 * 1.732_050_8;

/// A named screen element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    Help,
    Legend,
    TopBar,
    Net,
    Palette,
    Status,
    CameraBar,
    Preview,
    #[cfg(feature = "camera")]
    Hud,
    Cube,
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Box(Rect),
    Disc { center: Vec2, radius: f32 },
}

impl Shape {
    pub fn bounds(self) -> Rect {
        match self {
            Shape::Box(r) => r,
            Shape::Disc { center, radius } => {
                Rect::from_center_half_size(center, Vec2::splat(radius))
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Footprint {
    pub element: Element,
    pub shape: Shape,
}

impl Footprint {
    /// Whether two different elements cover common pixels (touching is fine).
    /// The palette sits in the net's empty corner by design.
    pub fn overlaps(&self, other: &Footprint) -> bool {
        if self.element == other.element
            || matches!(
                (self.element, other.element),
                (Element::Net, Element::Palette) | (Element::Palette, Element::Net)
            )
        {
            return false;
        }
        match (self.shape, other.shape) {
            (Shape::Box(a), Shape::Box(b)) => {
                a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
            }
            (Shape::Box(r), Shape::Disc { center, radius })
            | (Shape::Disc { center, radius }, Shape::Box(r)) => {
                center.clamp(r.min, r.max).distance(center) < radius
            }
            (Shape::Disc { .. }, Shape::Disc { .. }) => false,
        }
    }
}

/// Every visible element's footprint for `layout` in `state`'s window.
pub fn footprints(state: &ScreenState, layout: &FrameLayout) -> Vec<Footprint> {
    let size = state.size;
    let mut out = Vec::new();
    let mut add = |element, shape| out.push(Footprint { element, shape });

    if layout.desktop_text {
        let help = text_size(crate::ui::HELP, HELP_FONT, f32::INFINITY);
        add(Element::Help, Shape::Box(place(&help_edges(), help, size)));
        let legend = text_size(crate::axis::LEGEND, LEGEND_FONT, f32::INFINITY);
        add(
            Element::Legend,
            Shape::Box(place(&layout.legend, legend, size)),
        );
    }

    for row in top_bar_rows(&layout.top_bar, size) {
        add(Element::TopBar, Shape::Box(row));
    }

    if let Some(net) = layout.net {
        let origin = place(&net, Vec2::new(crate::net::NET_W, crate::net::NET_H), size).min;
        for face in Face::ALL {
            let (row, col) = crate::net::face_grid(face);
            let at = origin + Vec2::new(col as f32, row as f32) * crate::net::STRIDE;
            let block = Rect::from_corners(at, at + Vec2::splat(crate::net::BLOCK));
            add(Element::Net, Shape::Box(block));
        }
        if layout.palette {
            use crate::net::{NET_W, PALETTE_H, PALETTE_RIGHT_INSET, PALETTE_TOP, PALETTE_W};
            let at = origin + Vec2::new(NET_W - PALETTE_W - PALETTE_RIGHT_INSET, PALETTE_TOP);
            let palette = Rect::from_corners(at, at + Vec2::new(PALETTE_W, PALETTE_H));
            add(Element::Palette, Shape::Box(palette));
        }
    }

    let status_w = resolve(layout.status.max_width, size.x);
    let status = text_size(&longest_status(state.flow), STATUS_FONT, status_w);
    add(
        Element::Status,
        Shape::Box(place(&layout.status, status, size)),
    );

    if let Some(preview) = layout.preview {
        let node = Vec2::new(
            resolve(preview.width, size.x),
            resolve(preview.height, size.y),
        );
        add(Element::Preview, Shape::Box(place(&preview, node, size)));
    }

    if layout.camera_buttons {
        let bar = layout.camera_bar;
        let left = resolve(Some(bar.left), size.x);
        let width = resolve(bar.width, size.x);
        let rows = wrap_rows(
            &camera_button_sizes(),
            left,
            width,
            CAMERA_BAR_GAP,
            RowAlign::Center,
        );
        let top = size.y - resolve(Some(bar.bottom), size.y) - rows_height(&rows);
        for row in rows {
            let r = Rect::new(row.min.x, top + row.min.y, row.max.x, top + row.max.y);
            add(Element::CameraBar, Shape::Box(r));
        }
    }

    #[cfg(feature = "camera")]
    if let Some(hud) = layout.hud {
        let width = match hud.width {
            Some(Val::Px(w)) => w,
            _ => size.x - resolve(Some(hud.left), size.x) - resolve(Some(hud.right), size.x),
        };
        let text = crate::camera_scan::longest_hud_text(layout.compact);
        let h = text_size(&text, layout.hud_font, width - 2.0 * HUD_PAD).y + 2.0 * HUD_PAD;
        add(
            Element::Hud,
            Shape::Box(place(&hud, Vec2::new(width, h), size)),
        );
    }

    if let Some(cube) = layout.cube {
        let (center, radius) = project_cube(cube, size);
        add(Element::Cube, Shape::Disc { center, radius });
    }
    out
}

/// The help panel's fixed spot (see `ui::setup_ui`).
fn help_edges() -> Edges {
    Edges {
        left: Val::Px(EDGE),
        top: Val::Px(EDGE),
        ..Edges::AUTO
    }
}

/// The top bar's wrapped button rows, in window coordinates.
fn top_bar_rows(bar: &TopBar, window: Vec2) -> Vec<Rect> {
    let buttons: Vec<Vec2> = bar.controls.iter().map(|&c| top_button_size(c)).collect();
    let left = resolve(Some(bar.edges.left), window.x);
    let width = resolve(bar.edges.width, window.x);
    let top = resolve(Some(bar.edges.top), window.y);
    let align = if bar.justify == JustifyContent::Center {
        RowAlign::Center
    } else {
        RowAlign::Start
    };
    wrap_rows(&buttons, left, width, TOP_BAR_GAP, align)
        .into_iter()
        .map(|r| Rect::new(r.min.x, top + r.min.y, r.max.x, top + r.max.y))
        .collect()
}

/// The widest status each state can show (the status wraps within its width).
fn longest_status(flow: FlowKind) -> String {
    match flow {
        FlowKind::Picker => "INPUT · choose a setup method".into(),
        // The longest `CubeError` message.
        FlowKind::Editing => {
            "INPUT · impossible - the known stickers cannot form a solvable cube".into()
        }
        FlowKind::Scanning => "CAMERA · scanning…".into(),
        FlowKind::Solving => "SOLVE · Incomplete - 47/48 stickers known\nBeginner · 135/135 · step 7/7: Last layer edges  (playing)".into(),
    }
}

/// Resolve `edges` for a node of `node` size in a `window`.
fn place(edges: &Edges, node: Vec2, window: Vec2) -> Rect {
    let width = match (edges.left, edges.right) {
        (Val::Px(l), Val::Px(r)) if edges.width == Some(Val::Auto) => window.x - l - r,
        _ => node.x,
    };
    let x = match (edges.left, edges.right) {
        (Val::Px(l), _) => l,
        (_, Val::Px(r)) => window.x - r - width,
        _ => panic!("node has no horizontal anchor: {edges:?}"),
    };
    let y = match (edges.top, edges.bottom) {
        (Val::Px(t), _) => t,
        (_, Val::Px(b)) => window.y - b - node.y,
        _ => panic!("node has no vertical anchor: {edges:?}"),
    };
    Rect::new(x, y, x + width, y + node.y)
}

/// A length in px, resolving percentages against `axis` (the window's extent;
/// all these nodes are absolutely positioned against the window).
fn resolve(v: Option<Val>, axis: f32) -> f32 {
    match v {
        Some(Val::Px(p)) => p,
        Some(Val::Percent(p) | Val::Vw(p)) => axis * p / 100.0,
        other => panic!("expected a definite length, got {other:?}"),
    }
}

/// Screen-space center and radius of the cube's bounding sphere under `cube`'s
/// framing and the default orbit angles.
fn project_cube(cube: CubeFraming, window: Vec2) -> (Vec2, f32) {
    let orbit = OrbitCamera::DEFAULT;
    let focus = Vec3::new(0.0, cube.focus_y, 0.0);
    let rot = Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(-orbit.pitch);
    let eye = focus + rot * Vec3::new(0.0, 0.0, cube.radius);
    let forward = (focus - eye).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward);

    let to_cube = Vec3::ZERO - eye;
    let depth = to_cube.dot(forward);
    let half_h = (FOV_Y / 2.0).tan() * depth;
    let half_w = half_h * window.x / window.y;
    let ndc = Vec2::new(to_cube.dot(right) / half_w, to_cube.dot(up) / half_h);
    let center = Vec2::new(
        (ndc.x + 1.0) / 2.0 * window.x,
        (1.0 - ndc.y) / 2.0 * window.y,
    );
    let radius = CUBE_SPHERE / half_h * window.y / 2.0;
    (center, radius)
}
