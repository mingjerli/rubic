//! Move-engine and facelet behavior tests.
//!
//! These are chosen to pin down correctness without external golden data: the
//! order of `R U R' U'` (6) and `R U` (105) are classic, highly-sensitive
//! checks that catch subtle permutation errors.

use rubic_core::{Amount, Face, Facelets, Move, Sequence};

fn seq(s: &str) -> Sequence {
    s.parse().expect("valid notation")
}

#[test]
fn solved_string_is_canonical() {
    assert_eq!(
        Facelets::SOLVED.to_string(),
        "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB"
    );
}

#[test]
fn solved_round_trips_through_string() {
    let s = Facelets::SOLVED.to_string();
    assert_eq!(s.len(), 54);
    assert_eq!(s.parse::<Facelets>().unwrap(), Facelets::SOLVED);
}

#[test]
fn from_str_rejects_wrong_length() {
    assert!("UUU".parse::<Facelets>().is_err());
}

#[test]
fn from_str_rejects_invalid_char() {
    let bad = "X".repeat(54);
    assert!(bad.parse::<Facelets>().is_err());
}

#[test]
fn notation_round_trips() {
    let s = "R U R' U2 F D' L2 B";
    assert_eq!(seq(s).to_string(), s);
}

#[test]
fn quarter_turn_four_times_is_identity() {
    for face in Face::ALL {
        let m = Move {
            face,
            amount: Amount::Cw,
        };
        let mut c = Facelets::SOLVED;
        for _ in 0..4 {
            c = c.apply(m);
        }
        assert_eq!(c, Facelets::SOLVED, "{face:?} applied 4x should be solved");
    }
}

#[test]
fn move_then_inverse_is_identity() {
    for face in Face::ALL {
        for amount in [Amount::Cw, Amount::Ccw, Amount::Double] {
            let m = Move { face, amount };
            let c = Facelets::SOLVED.apply(m).apply(m.inverse());
            assert_eq!(c, Facelets::SOLVED, "{m} then inverse should be solved");
        }
    }
}

#[test]
fn double_equals_two_quarters() {
    for face in Face::ALL {
        let double = Facelets::SOLVED.apply(Move {
            face,
            amount: Amount::Double,
        });
        let twice = Facelets::SOLVED
            .apply(Move {
                face,
                amount: Amount::Cw,
            })
            .apply(Move {
                face,
                amount: Amount::Cw,
            });
        assert_eq!(double, twice, "{face:?}2 should equal {face:?} {face:?}");
    }
}

#[test]
fn centers_never_move() {
    let scrambled = Facelets::SOLVED.apply_seq(&seq("R U R' U' F2 L D B'"));
    for (i, face) in [
        (4, Face::U),
        (13, Face::R),
        (22, Face::F),
        (31, Face::D),
        (40, Face::L),
        (49, Face::B),
    ] {
        assert_eq!(scrambled.get(i), face, "center {i} moved");
    }
}

#[test]
fn color_counts_preserved_under_scramble() {
    let scrambled = Facelets::SOLVED.apply_seq(&seq("R U2 F' L D B R' U D2 F"));
    let s = scrambled.to_string();
    for ch in ['U', 'R', 'F', 'D', 'L', 'B'] {
        assert_eq!(
            s.chars().filter(|&x| x == ch).count(),
            9,
            "color {ch} count changed"
        );
    }
}

#[test]
fn sequence_inverse_undoes_scramble() {
    let scramble = seq("R U2 F' L D B R' U D2 F L' B2");
    let c = Facelets::SOLVED
        .apply_seq(&scramble)
        .apply_seq(&scramble.inverse());
    assert_eq!(c, Facelets::SOLVED);
}

#[test]
fn sexy_move_has_order_six() {
    let sexy = seq("R U R' U'");
    let mut c = Facelets::SOLVED;
    for i in 1..6 {
        c = c.apply_seq(&sexy);
        assert_ne!(c, Facelets::SOLVED, "solved too early after {i} sexy moves");
    }
    c = c.apply_seq(&sexy);
    assert_eq!(c, Facelets::SOLVED, "sexy move should have order 6");
}

#[test]
fn r_u_has_order_105() {
    let ru = seq("R U");
    let mut c = Facelets::SOLVED;
    for i in 1..105 {
        c = c.apply_seq(&ru);
        assert_ne!(c, Facelets::SOLVED, "R U solved too early after {i} reps");
    }
    c = c.apply_seq(&ru);
    assert_eq!(c, Facelets::SOLVED, "R U should have order 105");
}
