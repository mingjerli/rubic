//! Scan behaviour, plus reading helpers for other tests.

use super::*;
use crate::vision::capture::CAPTURE_ORDER;
use rubic_core::{Completion, Facelets};

/// A face's nine stickers in its ideal scheme color.
pub fn solid(face: Face) -> [Rgb; 9] {
    let c = sticker_rgb(face);
    [[
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
    ]; 9]
}

/// The reading of face `face` of a solved cube, as the camera would see it.
pub fn solved_reading(face: Face) -> [Rgb; 9] {
    std::array::from_fn(|k| {
        let c = sticker_rgb(Facelets::SOLVED.get(face.index() * 9 + k));
        [
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
        ]
    })
}

#[test]
fn capture_fills_the_live_net_with_the_face_in_view() {
    let mut scan = Scan::new();
    let face = scan.target().unwrap();
    scan.observe(Some(solid(face)));
    assert!(scan.capture());
    assert!(scan.current_captured());
    for k in 0..9 {
        assert_eq!(scan.live().get(face.index() * 9 + k), Some(face));
    }
}

#[test]
fn capture_with_nothing_in_view_captures_nothing() {
    let mut scan = Scan::new();
    scan.observe(None);
    assert!(!scan.capture());
    assert!(!scan.current_captured());
}

#[test]
fn capture_commits_the_latest_reading() {
    let mut scan = Scan::new();
    let face = scan.target().unwrap();
    let other = Face::ALL.into_iter().find(|&f| f != face).unwrap();
    scan.observe(Some(solid(other)));
    scan.observe(Some(solid(face)));
    scan.capture();
    assert_eq!(scan.live().get(face.index() * 9), Some(face));
}

#[test]
fn moving_faces_needs_a_fresh_reading() {
    let mut scan = Scan::new();
    scan.observe(Some(solid(scan.target().unwrap())));
    scan.capture();
    scan.next_face();
    assert!(!scan.in_view());
    assert!(
        !scan.capture(),
        "the previous face's reading must not be reused"
    );
}

#[test]
fn next_face_waits_for_a_capture() {
    let mut scan = Scan::new();
    assert!(scan.next_face().is_none());
    assert_eq!(scan.index(), 0);
}

#[test]
fn six_captures_hand_off_the_scanned_cube() {
    let mut scan = Scan::new();
    let mut handoff = None;
    for face in CAPTURE_ORDER {
        scan.observe(Some(solved_reading(face)));
        assert!(scan.capture());
        handoff = scan.next_face();
    }
    let partial = handoff.expect("the sixth Next hands off");
    assert!(matches!(partial.analyze(), Completion::Unique(_)));
}

#[test]
fn prev_face_goes_back_and_restart_clears_everything() {
    let mut scan = Scan::new();
    scan.observe(Some(solid(scan.target().unwrap())));
    scan.capture();
    scan.next_face();
    scan.prev_face();
    assert_eq!(scan.index(), 0);
    assert!(scan.current_captured());
    scan.restart();
    assert_eq!(scan.index(), 0);
    assert!(!scan.current_captured());
    assert_eq!(scan.live().known_count(), 0);
}

#[test]
fn camera_is_starting_until_the_first_image() {
    let mut scan = Scan::new();
    scan.tick(5.0, false);
    scan.tick(6.0, false);
    assert_eq!(scan.camera_status(), CameraStatus::Starting);
    scan.tick(6.1, true);
    assert_eq!(scan.camera_status(), CameraStatus::Live);
}

#[test]
fn camera_is_unavailable_when_no_image_ever_arrives() {
    let mut scan = Scan::new();
    scan.tick(5.0, false);
    scan.tick(5.0 + NO_IMAGE_GRACE_SECS + 0.1, false);
    assert_eq!(scan.camera_status(), CameraStatus::Unavailable);
}

#[test]
fn camera_is_unavailable_when_images_stop_and_live_when_they_resume() {
    let mut scan = Scan::new();
    scan.tick(0.0, true);
    scan.tick(1.0, false);
    assert_eq!(
        scan.camera_status(),
        CameraStatus::Live,
        "a short gap is fine"
    );
    scan.tick(1.0 + NO_IMAGE_GRACE_SECS + 0.1, false);
    assert_eq!(scan.camera_status(), CameraStatus::Unavailable);
    scan.tick(9.0, true);
    assert_eq!(scan.camera_status(), CameraStatus::Live);
}

#[test]
fn restarting_keeps_the_camera_live() {
    let mut scan = Scan::new();
    scan.tick(0.0, true);
    scan.restart();
    assert_eq!(scan.camera_status(), CameraStatus::Live);
}
