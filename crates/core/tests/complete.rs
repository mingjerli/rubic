//! Partial-input completion tests (the "minimal input / enough / impossible"
//! requirement).

use rubic_core::{Completion, CubeState, Face, Facelets, PartialFacelets, Sequence};

fn scramble(s: &str) -> Facelets {
    Facelets::SOLVED.apply_seq(&s.parse::<Sequence>().unwrap())
}

#[test]
fn full_solved_input_is_unique() {
    let p = PartialFacelets::from_facelets(&Facelets::SOLVED);
    match p.analyze() {
        Completion::Unique(s) => assert_eq!(s, CubeState::SOLVED),
        other => panic!("expected Unique, got {other:?}"),
    }
}

#[test]
fn full_scramble_input_is_unique() {
    let f = scramble("R U F2 L' D B");
    let p = PartialFacelets::from_facelets(&f);
    match p.analyze() {
        Completion::Unique(s) => assert_eq!(s.to_facelets(), f),
        other => panic!("expected Unique, got {other:?}"),
    }
}

#[test]
fn empty_input_needs_more() {
    let p = PartialFacelets::new();
    assert!(matches!(p.analyze(), Completion::NeedMore { .. }));
}

#[test]
fn five_faces_are_enough() {
    // Known everything except the whole B face (its 8 non-center stickers).
    let f = scramble("R U R' F2 D L");
    let mut p = PartialFacelets::from_facelets(&f);
    for i in 45..54 {
        p = p.clear(i);
    }
    assert!(
        matches!(p.analyze(), Completion::Unique(_)),
        "5 known faces should determine the 6th"
    );
}

#[test]
fn twisted_corner_full_input_is_impossible() {
    let mut co = [0u8; 8];
    co[0] = 1;
    let bad = CubeState::new_unchecked(
        [0, 1, 2, 3, 4, 5, 6, 7],
        co,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        [0; 12],
    )
    .to_facelets();
    let p = PartialFacelets::from_facelets(&bad);
    assert!(matches!(p.analyze(), Completion::Impossible(_)));
}

#[test]
fn too_many_of_one_color_is_impossible() {
    // Center R already counts once; add nine more R stickers -> 10 > 9.
    let mut p = PartialFacelets::new();
    for i in [0, 1, 2, 3, 5, 6, 7, 8, 45] {
        p = p.set(i, Face::R);
    }
    assert!(matches!(p.analyze(), Completion::Impossible(_)));
}

#[test]
fn known_count_tracks_non_center_stickers() {
    let p = PartialFacelets::new().set(0, Face::U).set(1, Face::R);
    assert_eq!(p.known_count(), 2);
}
