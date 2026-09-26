//! Sizes and spacings shared by the layout and the systems that spawn each
//! element, so placement and rendering can't drift apart.

use bevy::prelude::*;

/// Below this window width (logical px) the screen is a phone.
pub const NARROW_WIDTH: f32 = 720.0;
/// Portrait windows narrower than this (tablets) also use the compact layout.
pub const PORTRAIT_COMPACT_WIDTH: f32 = 1024.0;

/// Margin between most elements and the window edge.
pub const EDGE: f32 = 8.0;

/// Compact layout: top of the top control bar, and of the net below it.
pub const TOP_BAR_TOP: f32 = 48.0;
pub const NET_TOP_COMPACT: f32 = 96.0;
/// Desktop: gap between the help panel and the legend below it, and between
/// that text column and the top bar beside it.
pub const LEGEND_GAP: f32 = 4.0;
pub const TEXT_COLUMN_GAP: f32 = 16.0;

/// Camera preview: a 4:3 inset in the bottom-right corner, scaled to the window
/// width within limits.
pub const PREVIEW_FRACTION: f32 = 0.38;
pub const PREVIEW_MIN_W: f32 = 150.0;
pub const PREVIEW_MAX_W: f32 = 360.0;
pub const PREVIEW_ASPECT: f32 = 0.75;
pub const CORNER_MARGIN: f32 = 10.0;
/// Gap between the desktop HUD banner and the preview below it.
pub const HUD_GAP_ABOVE_PREVIEW: f32 = 6.0;
/// Gap between the compact HUD strip and the net above it.
pub const HUD_GAP_BELOW_NET: f32 = 12.0;
/// On desktop the status sits bottom-left beside the camera bar; reserve it.
pub const STATUS_RESERVE_W: f32 = 185.0;
/// The desktop camera bar never shrinks below this, even if it must wrap.
pub const CAMERA_BAR_MIN_W: f32 = 150.0;
/// Distance of the camera bar from the window's bottom edge.
pub const CAMERA_BAR_BOTTOM: f32 = 12.0;

/// Text sizes (logical px).
pub const HELP_FONT: f32 = 13.0;
pub const LEGEND_FONT: f32 = 12.0;
pub const STATUS_FONT: f32 = 16.0;
/// Desktop: the status line wraps at this share of the window width.
pub const STATUS_MAX_VW: f32 = 62.0;
/// HUD font size in the compact and desktop layouts.
pub const HUD_FONT_COMPACT: f32 = 12.0;
pub const HUD_FONT_WIDE: f32 = 16.0;
/// Padding inside the camera HUD banner.
#[cfg(feature = "camera")]
pub const HUD_PAD: f32 = 8.0;

/// Top-bar buttons: label, optional hint below it, padding, and flex gaps.
pub const TOP_BUTTON_FONT: f32 = 16.0;
pub const TOP_HINT_FONT: f32 = 11.0;
pub const TOP_BUTTON_PAD: Vec2 = Vec2::new(14.0, 9.0);
pub const TOP_BAR_GAP: Vec2 = Vec2::new(8.0, 8.0);
/// Camera-bar buttons: bigger touch targets.
#[cfg(feature = "camera")]
pub const CAMERA_BUTTON_FONT: f32 = 18.0;
#[cfg(feature = "camera")]
pub const CAMERA_BUTTON_PAD: Vec2 = Vec2::new(18.0, 12.0);
pub const CAMERA_BAR_GAP: Vec2 = Vec2::new(12.0, 8.0);
pub const BUTTON_BORDER: f32 = 1.0;

/// Orbit radius in the compact layout (a smaller, closer cube than desktop).
pub const COMPACT_RADIUS: f32 = 17.0;
/// How far to raise the camera focus so the cube renders below the net in the
/// compact layout while editing: lifting the focus drops the cube into the
/// space under the net.
pub const CUBE_DOWN_SHIFT: f32 = 4.0;
