//! What the Scan HUD says: like a check scanner, which face to present, how to
//! hold it, whether it's in view, and how to capture it. Pure, so every
//! message is tested; `camera_scan` only puts it on screen.
//!
//! ASCII only: the default font has no glyphs for `·`, `—` or `✓`.

use rubic_core::Face;

use super::{CameraStatus, Scan};

/// Guidance for a face: `(which face by center color, how to orient it)`.
///
/// The orientation cue is required: each face's stickers are filed into fixed
/// facelet slots, so the face must be held the right way up or its border
/// stickers land rotated. Derived from the core's facelet geometry (standard
/// URFDLB, white=U/green=F): side faces keep white up; the white face keeps
/// green toward the bottom; the yellow face keeps green toward the top.
fn face_hint(face: Face) -> (&'static str, &'static str) {
    match face {
        Face::U => ("WHITE face", "keep the GREEN side at the BOTTOM"),
        Face::R => ("RED face", "keep WHITE on top"),
        Face::F => ("GREEN face", "keep WHITE on top"),
        Face::D => ("YELLOW face", "keep the GREEN side at the TOP"),
        Face::L => ("ORANGE face", "keep WHITE on top"),
        Face::B => ("BLUE face", "keep WHITE on top"),
    }
}

const STARTING: &str = "Starting the camera...";
const UNAVAILABLE: &str = "No camera image\nAllow camera access, or Start over";
const KEYS: &str = "ENTER capture/retake | N next | P prev | R restart | Esc cancel";

/// The one-line capture status for the face being presented.
fn capture_status(captured: bool, in_view: bool) -> &'static str {
    if captured {
        "Captured - Next when happy"
    } else if in_view {
        "In view - Capture now"
    } else {
        "Line the face up in the box"
    }
}

fn face_text(face: Face, index: usize, status: &str, compact: bool) -> String {
    let (name, orient) = face_hint(face);
    let mut s = format!("Face {}/6: {name}\n{orient}\n{status}", index + 1);
    // Keyboard hints only make sense on desktop; the buttons carry those
    // actions in the compact layout.
    if !compact {
        s.push('\n');
        s.push_str(KEYS);
    }
    s
}

/// The HUD text for `scan`.
#[must_use]
pub fn hud_text(scan: &Scan, compact: bool) -> String {
    match scan.camera_status() {
        CameraStatus::Starting => STARTING.to_string(),
        CameraStatus::Unavailable => UNAVAILABLE.to_string(),
        CameraStatus::Live => match scan.target() {
            Some(face) => face_text(
                face,
                scan.index(),
                capture_status(scan.current_captured(), scan.in_view()),
                compact,
            ),
            None => "Scan complete.".to_string(),
        },
    }
}

/// Every HUD text a Scan can show, for layout sizing (the layout wraps each
/// and reserves room for the tallest).
#[cfg(test)]
#[must_use]
pub fn every_hud_text(compact: bool) -> Vec<String> {
    let statuses = [
        capture_status(true, false),
        capture_status(false, true),
        capture_status(false, false),
    ];
    Face::ALL
        .into_iter()
        .flat_map(|face| statuses.map(|status| face_text(face, 5, status, compact)))
        .chain([
            STARTING.to_string(),
            UNAVAILABLE.to_string(),
            "Scan complete.".to_string(),
        ])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::tests::solid;

    fn live_scan() -> Scan {
        let mut scan = Scan::new();
        scan.tick(0.0, true);
        scan
    }

    #[test]
    fn says_starting_until_the_first_frame() {
        let mut scan = Scan::new();
        scan.tick(0.0, false);
        assert_eq!(hud_text(&scan, false), STARTING);
    }

    #[test]
    fn asks_for_camera_access_when_no_image_arrives() {
        let mut scan = Scan::new();
        scan.tick(0.0, false);
        scan.tick(10.0, false);
        assert!(hud_text(&scan, false).contains("Allow camera access"));
    }

    #[test]
    fn guides_the_first_face_and_its_orientation() {
        let text = hud_text(&live_scan(), true);
        assert!(text.starts_with("Face 1/6: WHITE face"), "{text}");
        assert!(text.contains("GREEN side at the BOTTOM"), "{text}");
        assert!(text.contains("Line the face up"), "{text}");
    }

    #[test]
    fn says_in_view_then_captured() {
        let mut scan = live_scan();
        scan.observe(Some(solid(scan.target().unwrap())));
        assert!(hud_text(&scan, true).contains("In view - Capture now"));
        scan.capture();
        assert!(hud_text(&scan, true).contains("Captured - Next when happy"));
    }

    #[test]
    fn keyboard_hints_only_on_desktop() {
        assert!(hud_text(&live_scan(), false).contains("ENTER capture"));
        assert!(!hud_text(&live_scan(), true).contains("ENTER capture"));
    }

    #[test]
    fn every_message_is_ascii() {
        for compact in [false, true] {
            let mut scan = live_scan();
            for _ in 0..3 {
                assert!(hud_text(&scan, compact).is_ascii());
                scan.observe(Some(solid(scan.target().unwrap())));
                assert!(hud_text(&scan, compact).is_ascii());
                scan.capture();
                assert!(hud_text(&scan, compact).is_ascii());
                scan.next_face();
            }
        }
        assert!(STARTING.is_ascii() && UNAVAILABLE.is_ascii());
    }
}
