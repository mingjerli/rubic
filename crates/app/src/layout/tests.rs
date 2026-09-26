use super::footprint::{Footprint, Shape, footprints};
use super::*;
use crate::mode::{AppMode, InputStage};
use crate::touch::TouchControl;

/// Window sizes every layout must work at: small and large phones, a portrait
/// tablet, and common desktop windows.
const SIZES: [(f32, f32); 6] = [
    (360.0, 740.0),
    (390.0, 844.0),
    (768.0, 1024.0),
    (1000.0, 760.0),
    (1280.0, 720.0),
    (1920.0, 1080.0),
];

/// Every reachable app state. The camera is only ever on while scanning (every
/// way out of a scan closes it).
fn states() -> Vec<(AppMode, InputStage, bool)> {
    let mut all = vec![
        (AppMode::Input, InputStage::ChooseMethod, false),
        (AppMode::Input, InputStage::Editing, false),
        (AppMode::Solve, InputStage::Editing, false),
    ];
    if cfg!(feature = "camera") {
        all.push((AppMode::Camera, InputStage::ChooseMethod, true));
    }
    all
}

fn screen(w: f32, h: f32, mode: AppMode, stage: InputStage, camera_on: bool) -> ScreenState {
    ScreenState {
        size: Vec2::new(w, h),
        mode,
        stage,
        camera_on,
    }
}

fn phone(mode: AppMode, stage: InputStage) -> FrameLayout {
    frame_layout(&screen(390.0, 844.0, mode, stage, mode == AppMode::Camera))
}

fn desktop(mode: AppMode, stage: InputStage) -> FrameLayout {
    frame_layout(&screen(1280.0, 720.0, mode, stage, mode == AppMode::Camera))
}

// --- The invariant the overlap fixes kept chasing ---------------------------

#[test]
fn no_visible_elements_overlap_in_any_state_or_size() {
    let mut clashes = Vec::new();
    for (w, h) in SIZES {
        for (mode, stage, camera_on) in states() {
            let state = screen(w, h, mode, stage, camera_on);
            let prints = footprints(&state, &frame_layout(&state));
            for (i, a) in prints.iter().enumerate() {
                for b in &prints[i + 1..] {
                    if a.overlaps(b) {
                        clashes.push(format!(
                            "{w}x{h} {mode:?}/{stage:?}: {:?} overlaps {:?}",
                            a.element, b.element
                        ));
                    }
                }
            }
        }
    }
    assert!(clashes.is_empty(), "overlaps:\n{}", clashes.join("\n"));
}

#[test]
fn everything_visible_stays_inside_the_window() {
    let mut outside = Vec::new();
    for (w, h) in SIZES {
        for (mode, stage, camera_on) in states() {
            let state = screen(w, h, mode, stage, camera_on);
            let window = Rect::new(0.0, 0.0, w, h);
            for Footprint { element, shape } in footprints(&state, &frame_layout(&state)) {
                let bounds = shape.bounds();
                if window.union(bounds) != window {
                    outside.push(format!(
                        "{w}x{h} {mode:?}/{stage:?}: {element:?} {bounds:?}"
                    ));
                }
            }
        }
    }
    assert!(outside.is_empty(), "off-screen:\n{}", outside.join("\n"));
}

// --- Phone vs desktop -------------------------------------------------------

#[test]
fn compact_below_720_logical_px() {
    let at = |w| {
        frame_layout(&screen(
            w,
            600.0,
            AppMode::Solve,
            InputStage::Editing,
            false,
        ))
    };
    assert!(at(719.0).compact);
    assert!(!at(720.0).compact);
}

#[test]
fn portrait_tablets_are_compact_landscape_ones_are_not() {
    assert!(is_compact(Vec2::new(768.0, 1024.0)));
    assert!(is_compact(Vec2::new(820.0, 1180.0)));
    assert!(!is_compact(Vec2::new(1024.0, 768.0)));
    assert!(!is_compact(Vec2::new(1024.0, 1366.0)));
}

#[test]
fn desktop_top_bar_sits_beside_the_reference_text() {
    let l = desktop(AppMode::Solve, InputStage::Editing);
    let Val::Px(left) = l.top_bar.edges.left else {
        panic!("desktop top bar should be placed in px");
    };
    // Clear of whichever reference text is wider.
    let help = text::text_size(crate::ui::HELP, HELP_FONT, f32::INFINITY);
    let legend = text::text_size(crate::axis::LEGEND, LEGEND_FONT, f32::INFINITY);
    assert_eq!(left, EDGE + help.x.max(legend.x) + TEXT_COLUMN_GAP);
    assert_eq!(l.top_bar.edges.top, Val::Px(EDGE));
    assert_eq!(l.top_bar.justify, JustifyContent::FlexStart);
}

#[test]
fn legend_sits_below_the_help_panel() {
    let l = desktop(AppMode::Solve, InputStage::Editing);
    let help = text::text_size(crate::ui::HELP, HELP_FONT, f32::INFINITY);
    assert_eq!(l.legend.top, Val::Px(EDGE + help.y + LEGEND_GAP));
}

#[test]
fn help_text_uses_only_glyphs_the_default_font_has() {
    for text in [crate::ui::HELP, crate::axis::LEGEND] {
        assert!(text.is_ascii(), "non-ASCII glyph renders as tofu: {text}");
    }
}

#[test]
fn desktop_reference_text_only_on_wide_screens() {
    assert!(!phone(AppMode::Solve, InputStage::Editing).desktop_text);
    assert!(!phone(AppMode::Solve, InputStage::Editing).axes);
    assert!(desktop(AppMode::Solve, InputStage::Editing).desktop_text);
}

#[test]
fn hud_text_is_smaller_on_phones() {
    assert!(
        phone(AppMode::Camera, InputStage::ChooseMethod).hud_font
            < desktop(AppMode::Camera, InputStage::ChooseMethod).hud_font
    );
}

// --- Which elements show in which state -------------------------------------

#[test]
fn net_shows_only_while_editing_or_scanning() {
    use InputStage::{ChooseMethod, Editing};
    // Method picker: hidden (a solved 3D preview stands in).
    assert!(desktop(AppMode::Input, ChooseMethod).net.is_none());
    // Editing by hand / reviewing a scan: shown.
    assert!(desktop(AppMode::Input, Editing).net.is_some());
    // A Scan fills the net live: it is the scan's progress view.
    assert!(desktop(AppMode::Camera, ChooseMethod).net.is_some());
    // Solving: only the 3D cube.
    assert!(desktop(AppMode::Solve, Editing).net.is_none());
}

#[test]
fn palette_shows_only_while_painting() {
    use InputStage::{ChooseMethod, Editing};
    assert!(desktop(AppMode::Input, Editing).palette);
    assert!(!desktop(AppMode::Input, ChooseMethod).palette);
    assert!(!desktop(AppMode::Camera, Editing).palette);
    assert!(!desktop(AppMode::Solve, Editing).palette);
}

#[test]
fn method_picker_offers_the_setup_methods() {
    let bar = desktop(AppMode::Input, InputStage::ChooseMethod)
        .top_bar
        .controls;
    assert!(bar.contains(&TouchControl::NewGame));
    assert!(bar.contains(&TouchControl::Manual));
    assert_eq!(
        bar.contains(&TouchControl::Camera),
        cfg!(feature = "camera")
    );
    assert!(!bar.contains(&TouchControl::Solve));
    assert!(!bar.contains(&TouchControl::StartOver));
}

#[test]
fn editing_offers_solve_and_start_over() {
    let bar = desktop(AppMode::Input, InputStage::Editing)
        .top_bar
        .controls;
    assert_eq!(bar, vec![TouchControl::Solve, TouchControl::StartOver]);
}

#[test]
fn solve_offers_shuffle_edit_solvers_and_playback() {
    use TouchControl::{Beginner, Edit, NewGame, Next, Optimal, Play, Prev};
    let bar = desktop(AppMode::Solve, InputStage::Editing)
        .top_bar
        .controls;
    assert_eq!(
        bar,
        vec![NewGame, Edit, Beginner, Optimal, Prev, Play, Next]
    );
}

#[test]
fn scan_uses_the_camera_bar_not_the_top_bar() {
    let l = desktop(AppMode::Camera, InputStage::ChooseMethod);
    assert!(l.top_bar.controls.is_empty());
    assert!(l.camera_buttons);
    assert!(l.hud.is_some());
    assert!(!desktop(AppMode::Solve, InputStage::Editing).camera_buttons);
    assert!(desktop(AppMode::Solve, InputStage::Editing).hud.is_none());
}

#[test]
fn cube_and_axes_hide_during_a_scan() {
    let scan = desktop(AppMode::Camera, InputStage::ChooseMethod);
    assert!(scan.cube.is_none());
    assert!(!scan.axes);
    assert!(desktop(AppMode::Solve, InputStage::Editing).axes);
    // Axes would poke into the net on a phone.
    assert!(!phone(AppMode::Solve, InputStage::Editing).axes);
}

#[test]
fn preview_shows_while_the_camera_is_on_except_when_solving() {
    let on = |mode, stage| {
        frame_layout(&screen(1280.0, 720.0, mode, stage, true))
            .preview
            .is_some()
    };
    assert!(on(AppMode::Camera, InputStage::ChooseMethod));
    assert!(on(AppMode::Input, InputStage::Editing));
    assert!(!on(AppMode::Solve, InputStage::Editing));
    let off = frame_layout(&screen(
        1280.0,
        720.0,
        AppMode::Camera,
        InputStage::ChooseMethod,
        false,
    ));
    assert!(off.preview.is_none());
}

// --- Placement --------------------------------------------------------------

#[test]
fn compact_net_is_centered_below_the_top_bar() {
    let net = phone(AppMode::Input, InputStage::Editing).net.unwrap();
    assert_eq!(net.left, Val::Px((390.0 - crate::net::NET_W) / 2.0));
    assert_eq!(net.top, Val::Px(NET_TOP_COMPACT));
}

#[test]
fn desktop_net_is_tucked_top_right() {
    let net = desktop(AppMode::Input, InputStage::Editing).net.unwrap();
    assert_eq!(net.right, Val::Px(EDGE));
    assert_eq!(net.top, Val::Px(EDGE));
    assert_eq!(net.left, Val::Auto);
}

#[test]
fn compact_status_moves_to_the_bottom_only_when_solving() {
    let top = phone(AppMode::Input, InputStage::Editing).status;
    assert_eq!((top.top, top.bottom), (Val::Px(EDGE), Val::Auto));
    let bottom = phone(AppMode::Solve, InputStage::Editing).status;
    assert_eq!((bottom.top, bottom.bottom), (Val::Auto, Val::Px(EDGE)));
    let desk = desktop(AppMode::Input, InputStage::Editing).status;
    assert_eq!((desk.top, desk.bottom), (Val::Auto, Val::Px(EDGE)));
}

#[test]
fn preview_scales_with_the_window_within_limits() {
    let width = |w| {
        frame_layout(&screen(
            w,
            800.0,
            AppMode::Camera,
            InputStage::ChooseMethod,
            true,
        ))
        .preview
        .unwrap()
        .width
    };
    assert_eq!(width(300.0), Some(Val::Px(PREVIEW_MIN_W)));
    assert_eq!(width(500.0), Some(Val::Px(500.0 * PREVIEW_FRACTION)));
    assert_eq!(width(1920.0), Some(Val::Px(PREVIEW_MAX_W)));
}

#[test]
fn compact_hud_is_a_strip_below_the_net() {
    let hud = phone(AppMode::Camera, InputStage::ChooseMethod)
        .hud
        .unwrap();
    assert_eq!(hud.left, Val::Px(EDGE));
    assert_eq!(hud.right, Val::Px(EDGE));
    assert_eq!(
        hud.top,
        Val::Px(NET_TOP_COMPACT + crate::net::NET_H + HUD_GAP_BELOW_NET)
    );
}

#[cfg(feature = "camera")]
#[test]
fn compact_preview_stacks_above_a_full_width_camera_bar() {
    let l = phone(AppMode::Camera, InputStage::ChooseMethod);
    assert_eq!(l.camera_bar.width, Some(Val::Percent(100.0)));
    let Val::Px(bottom) = l.preview.unwrap().bottom else {
        panic!("preview should be anchored to the bottom in px");
    };
    assert!(bottom > CAMERA_BAR_BOTTOM + 40.0, "preview bottom {bottom}");
}

#[test]
fn compact_status_at_the_top_uses_the_full_width() {
    let status = phone(AppMode::Input, InputStage::Editing).status;
    assert_eq!(status.max_width, Some(Val::Px(390.0 - 2.0 * EDGE)));
}

#[test]
fn camera_bar_leaves_room_for_the_preview() {
    let l = desktop(AppMode::Camera, InputStage::ChooseMethod);
    let Val::Px(bar_w) = l.camera_bar.width.unwrap() else {
        panic!("camera bar width should be in px while the camera is on");
    };
    let Val::Px(preview_w) = l.preview.unwrap().width.unwrap() else {
        panic!("preview width should be in px");
    };
    assert!(bar_w + preview_w <= 1280.0);
    let off = desktop(AppMode::Solve, InputStage::Editing);
    assert_eq!(off.camera_bar.width, Some(Val::Percent(100.0)));
}

#[test]
fn cube_framing_is_closer_on_phones_and_drops_below_the_net_while_editing() {
    let desk = desktop(AppMode::Solve, InputStage::Editing).cube.unwrap();
    let solve = phone(AppMode::Solve, InputStage::Editing).cube.unwrap();
    let edit = phone(AppMode::Input, InputStage::Editing).cube.unwrap();
    assert!(solve.radius > desk.radius);
    assert_eq!(solve.focus_y, 0.0);
    assert!(edit.focus_y > 0.0);
}

// --- The cube's projected footprint -----------------------------------------

#[test]
fn centered_cube_projects_to_the_window_center() {
    let state = screen(1280.0, 720.0, AppMode::Solve, InputStage::Editing, false);
    let cube = footprints(&state, &frame_layout(&state))
        .into_iter()
        .find_map(|f| match f.shape {
            Shape::Disc { center, .. } => Some(center),
            Shape::Box(_) => None,
        })
        .unwrap();
    assert!((cube - Vec2::new(640.0, 360.0)).length() < 1.0, "{cube:?}");
}

#[test]
fn raising_the_focus_drops_the_cube_down_the_screen() {
    let state = screen(390.0, 844.0, AppMode::Input, InputStage::Editing, false);
    let center = footprints(&state, &frame_layout(&state))
        .into_iter()
        .find_map(|f| match f.shape {
            Shape::Disc { center, .. } => Some(center),
            Shape::Box(_) => None,
        })
        .unwrap();
    assert!(center.y > 844.0 / 2.0 + 100.0, "{center:?}");
}
