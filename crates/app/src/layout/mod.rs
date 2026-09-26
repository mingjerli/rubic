//! Screen layout: where every 2D element sits, and whether it shows, for a
//! window size and app state.
//!
//! [`frame_layout`] is the single source of truth for placement. It is pure, so
//! the whole matrix of (window size x state) is tested for overlaps without
//! running Bevy. [`update_frame_layout`] recomputes it into the [`FrameLayout`]
//! resource each frame; the per-element systems only apply their slice of it.
//!
//! Two layouts: **compact** (phones and portrait tablets) stacks the top bar,
//! net, and cube vertically; **desktop** puts the keyboard reference text
//! top-left with the top bar beside it, and the net top-right.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::flow::{Flow, FlowKind};
use crate::net::{NET_H, NET_W};
use crate::touch::TouchControl;
use crate::types::OrbitCamera;

mod metrics;
pub mod text;

#[cfg(test)]
mod footprint;
#[cfg(test)]
mod tests;

pub use metrics::*;
#[cfg(feature = "camera")]
use text::button_size;
use text::{RowAlign, rows_height, text_size, wrap_rows};

/// The inputs the layout depends on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenState {
    /// Window size in logical px.
    pub size: Vec2,
    pub flow: FlowKind,
    /// Whether a camera source is open.
    pub camera_on: bool,
}

/// Where a UI node sits. `None` sizes leave the node's own size alone (text,
/// or a size fixed at spawn).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edges {
    pub left: Val,
    pub right: Val,
    pub top: Val,
    pub bottom: Val,
    pub width: Option<Val>,
    pub height: Option<Val>,
    pub max_width: Option<Val>,
}

impl Edges {
    const AUTO: Edges = Edges {
        left: Val::Auto,
        right: Val::Auto,
        top: Val::Auto,
        bottom: Val::Auto,
        width: None,
        height: None,
        max_width: None,
    };

    /// Write these edges into `node`, touching only fields that differ (so
    /// Bevy's change detection doesn't re-layout every frame).
    pub fn apply(&self, node: &mut Node) {
        set(&mut node.left, self.left);
        set(&mut node.right, self.right);
        set(&mut node.top, self.top);
        set(&mut node.bottom, self.bottom);
        for (slot, want) in [
            (&mut node.width, self.width),
            (&mut node.height, self.height),
            (&mut node.max_width, self.max_width),
        ] {
            if let Some(want) = want {
                set(slot, want);
            }
        }
    }
}

fn set(slot: &mut Val, want: Val) {
    if *slot != want {
        *slot = want;
    }
}

/// How the 3D camera frames the cube.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubeFraming {
    /// Orbit distance from the focus.
    pub radius: f32,
    /// World-space height of the orbit focus (the cube sits at the origin).
    pub focus_y: f32,
}

/// The top control bar: its buttons, where the bar sits, and how its rows align.
#[derive(Clone, Debug, PartialEq)]
pub struct TopBar {
    /// Buttons in display order (empty = bar hidden).
    pub controls: Vec<TouchControl>,
    pub edges: Edges,
    pub justify: JustifyContent,
}

/// Placement and visibility of every screen element for one frame. `None`
/// means hidden.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct FrameLayout {
    /// Phone or portrait tablet: stacked layout, no keyboard reference text.
    pub compact: bool,
    /// The keyboard help panel and orientation legend (desktop only).
    pub desktop_text: bool,
    pub legend: Edges,
    pub top_bar: TopBar,
    pub net: Option<Edges>,
    /// The palette rides inside the net, so it only needs a visibility flag.
    pub palette: bool,
    pub status: Edges,
    /// The camera-scan button bar's position (applied even while hidden).
    pub camera_bar: Edges,
    pub camera_buttons: bool,
    pub preview: Option<Edges>,
    pub hud: Option<Edges>,
    pub hud_font: f32,
    pub cube: Option<CubeFraming>,
    pub axes: bool,
}

impl Default for FrameLayout {
    fn default() -> Self {
        frame_layout(&ScreenState {
            size: Vec2::new(1280.0, 720.0),
            flow: FlowKind::Picker,
            camera_on: false,
        })
    }
}

/// Whether `size` gets the compact (stacked) layout.
#[must_use]
pub fn is_compact(size: Vec2) -> bool {
    size.x < NARROW_WIDTH || (size.y > size.x && size.x < PORTRAIT_COMPACT_WIDTH)
}

/// Lay out every element for `state`.
#[must_use]
pub fn frame_layout(state: &ScreenState) -> FrameLayout {
    let ScreenState {
        size,
        flow,
        camera_on,
    } = *state;
    let compact = is_compact(size);
    let editing = flow == FlowKind::Editing;
    let scanning = flow == FlowKind::Scanning;
    let preview_bottom = if compact {
        CAMERA_BAR_BOTTOM + camera_bar_height(size.x) + CAMERA_BAR_GAP.y
    } else {
        CORNER_MARGIN
    };

    FrameLayout {
        compact,
        desktop_text: !compact,
        legend: legend_edges(),
        top_bar: top_bar(size.x, compact, flow),
        net: (editing || scanning).then(|| net_edges(size.x, compact)),
        palette: editing,
        status: status_edges(size.x, compact, flow),
        camera_bar: camera_bar_edges(size.x, compact, camera_on),
        camera_buttons: scanning,
        preview: (camera_on && flow != FlowKind::Solving)
            .then(|| preview_edges(size.x, preview_bottom)),
        hud: scanning.then(|| hud_edges(size.x, compact)),
        hud_font: if compact {
            HUD_FONT_COMPACT
        } else {
            HUD_FONT_WIDE
        },
        cube: (!scanning).then_some(CubeFraming {
            radius: if compact {
                COMPACT_RADIUS
            } else {
                OrbitCamera::DEFAULT.radius
            },
            focus_y: if compact && editing {
                CUBE_DOWN_SHIFT
            } else {
                0.0
            },
        }),
        axes: !scanning && !compact,
    }
}

fn help_size() -> Vec2 {
    text_size(crate::ui::HELP, HELP_FONT, f32::INFINITY)
}

fn legend_size() -> Vec2 {
    text_size(crate::axis::LEGEND, LEGEND_FONT, f32::INFINITY)
}

/// Desktop: the legend sits just below the help panel.
fn legend_edges() -> Edges {
    Edges {
        left: Val::Px(EDGE),
        top: Val::Px(EDGE + help_size().y + LEGEND_GAP),
        ..Edges::AUTO
    }
}

/// Compact: a centered, full-width bar near the top. Desktop: at the top, left-
/// aligned just right of the reference text column, wrapping as needed.
fn top_bar(width: f32, compact: bool, flow: FlowKind) -> TopBar {
    let controls = top_bar_controls(flow);
    if compact {
        return TopBar {
            controls,
            edges: Edges {
                left: Val::Px(0.0),
                top: Val::Px(TOP_BAR_TOP),
                width: Some(Val::Percent(100.0)),
                ..Edges::AUTO
            },
            justify: JustifyContent::Center,
        };
    }
    let left = EDGE + help_size().x.max(legend_size().x) + TEXT_COLUMN_GAP;
    TopBar {
        controls,
        edges: Edges {
            left: Val::Px(left),
            top: Val::Px(EDGE),
            width: Some(Val::Px((width - left - EDGE).max(0.0))),
            ..Edges::AUTO
        },
        justify: JustifyContent::FlexStart,
    }
}

/// The top bar's controls. The method picker offers the Setup methods; editing
/// offers Solve + Start over; Solve offers Shuffle, Edit, solvers and playback.
/// A Scan uses its own bottom bar, so the top bar is empty there.
fn top_bar_controls(flow: FlowKind) -> Vec<TouchControl> {
    use TouchControl::{
        Beginner, Camera, Edit, Manual, Next, Optimal, Play, Prev, Shuffle, Solve, StartOver,
    };
    let shown = |control: TouchControl| match flow {
        // The Camera method only works with the `camera` feature.
        FlowKind::Picker => {
            matches!(control, Shuffle | Manual) || (control == Camera && cfg!(feature = "camera"))
        }
        FlowKind::Editing => matches!(control, Solve | StartOver),
        FlowKind::Solving => matches!(
            control,
            Shuffle | Edit | Beginner | Optimal | Prev | Play | Next
        ),
        FlowKind::Scanning => false,
    };
    TouchControl::ALL
        .into_iter()
        .filter(|&c| shown(c))
        .collect()
}

/// Compact: centered below the top bar. Desktop: tucked top-right.
fn net_edges(width: f32, compact: bool) -> Edges {
    if compact {
        Edges {
            left: Val::Px(((width - NET_W) / 2.0).max(4.0)),
            top: Val::Px(NET_TOP_COMPACT),
            ..Edges::AUTO
        }
    } else {
        Edges {
            right: Val::Px(EDGE),
            top: Val::Px(EDGE),
            ..Edges::AUTO
        }
    }
}

/// Compact: top-left, full width, except while solving (the bottom holds a
/// button bar, and in Solve the top is full of playback buttons instead).
/// Desktop: bottom-left.
fn status_edges(width: f32, compact: bool, flow: FlowKind) -> Edges {
    if compact && flow != FlowKind::Solving {
        Edges {
            left: Val::Px(EDGE),
            top: Val::Px(EDGE),
            max_width: Some(Val::Px(width - 2.0 * EDGE)),
            ..Edges::AUTO
        }
    } else {
        Edges {
            left: Val::Px(EDGE),
            bottom: Val::Px(EDGE),
            max_width: Some(Val::Vw(STATUS_MAX_VW)),
            ..Edges::AUTO
        }
    }
}

fn preview_width(width: f32) -> f32 {
    (width * PREVIEW_FRACTION).clamp(PREVIEW_MIN_W, PREVIEW_MAX_W)
}

fn preview_edges(width: f32, bottom: f32) -> Edges {
    let w = preview_width(width);
    Edges {
        right: Val::Px(CORNER_MARGIN),
        bottom: Val::Px(bottom),
        width: Some(Val::Px(w)),
        height: Some(Val::Px(w * PREVIEW_ASPECT)),
        ..Edges::AUTO
    }
}

/// Compact: a full-width strip between the net and the bottom controls (the
/// preview column is too narrow for the instructions). Desktop: a banner
/// directly above the preview, same right edge and width.
fn hud_edges(width: f32, compact: bool) -> Edges {
    if compact {
        Edges {
            left: Val::Px(EDGE),
            right: Val::Px(EDGE),
            top: Val::Px(NET_TOP_COMPACT + NET_H + HUD_GAP_BELOW_NET),
            width: Some(Val::Auto),
            ..Edges::AUTO
        }
    } else {
        let w = preview_width(width);
        Edges {
            right: Val::Px(CORNER_MARGIN),
            bottom: Val::Px(CORNER_MARGIN + w * PREVIEW_ASPECT + HUD_GAP_ABOVE_PREVIEW),
            width: Some(Val::Px(w)),
            ..Edges::AUTO
        }
    }
}

/// Desktop with the camera on: the bar sits between the bottom-left status and
/// the preview. Otherwise (and always in the compact layout, where the preview
/// stacks above it) the bar spans the window.
fn camera_bar_edges(width: f32, compact: bool, camera_on: bool) -> Edges {
    let (left, bar_width) = if camera_on && !compact {
        let preview = preview_width(width) + CORNER_MARGIN * 2.0;
        (
            Val::Px(STATUS_RESERVE_W),
            Val::Px((width - preview - STATUS_RESERVE_W).max(CAMERA_BAR_MIN_W)),
        )
    } else {
        (Val::Px(0.0), Val::Percent(100.0))
    };
    Edges {
        left,
        bottom: Val::Px(CAMERA_BAR_BOTTOM),
        width: Some(bar_width),
        ..Edges::AUTO
    }
}

/// Size of each camera-bar button, in display order.
#[must_use]
pub fn camera_button_sizes() -> Vec<Vec2> {
    #[cfg(feature = "camera")]
    {
        crate::camera_scan::CamButton::ALL
            .iter()
            .map(|b| {
                button_size(
                    b.label(),
                    None,
                    CAMERA_BUTTON_FONT,
                    CAMERA_BUTTON_PAD,
                    BUTTON_BORDER,
                )
            })
            .collect()
    }
    #[cfg(not(feature = "camera"))]
    Vec::new()
}

/// Height of the camera bar when it spans a window `width` wide.
fn camera_bar_height(width: f32) -> f32 {
    let rows = wrap_rows(
        &camera_button_sizes(),
        0.0,
        width,
        CAMERA_BAR_GAP,
        RowAlign::Center,
    );
    rows_height(&rows)
}

/// Recompute the [`FrameLayout`] resource from the window and app state.
pub fn update_frame_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    flow: Res<Flow>,
    #[cfg(feature = "camera")] feed: NonSend<crate::camera_scan::CameraFeed>,
    mut layout: ResMut<FrameLayout>,
) {
    let Ok(win) = windows.single() else {
        return;
    };
    #[cfg(feature = "camera")]
    let camera_on = feed.0.is_some();
    #[cfg(not(feature = "camera"))]
    let camera_on = false;
    let next = frame_layout(&ScreenState {
        size: win.size(),
        flow: flow.kind(),
        camera_on,
    });
    if *layout != next {
        *layout = next;
    }
}

/// Map a "should show" flag to a Bevy visibility.
#[must_use]
pub fn visibility(show: bool) -> Visibility {
    if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}
