//! Cheat-sheet generation tests.
//!
//! Besides checking that every stage is rendered, `every_algorithm_is_valid`
//! parses each printed algorithm through the real notation parser, so the cheat
//! sheet can never ship an unparseable/typo'd move sequence.

use rubic_core::Sequence;
use rubic_core::cheatsheet::{self, STAGES};

#[test]
fn has_seven_stages() {
    assert_eq!(STAGES.len(), 7);
}

#[test]
fn every_algorithm_is_valid_notation() {
    for stage in &STAGES {
        for (name, notation) in stage.algorithms {
            assert!(
                notation.parse::<Sequence>().is_ok(),
                "invalid notation in {}: {name} = {notation}",
                stage.title
            );
        }
    }
}

#[test]
fn markdown_covers_all_stages_and_algorithms() {
    let md = cheatsheet::markdown();
    for stage in &STAGES {
        assert!(md.contains(stage.title), "markdown missing {}", stage.title);
        assert!(
            md.contains(stage.theory),
            "markdown missing theory for {}",
            stage.title
        );
        for (_, notation) in stage.algorithms {
            assert!(
                md.contains(notation),
                "markdown missing algorithm {notation}"
            );
        }
    }
}

#[test]
fn html_is_self_contained_printable_document() {
    let h = cheatsheet::html();
    let lower = h.to_lowercase();
    assert!(lower.contains("<!doctype html"), "not an HTML document");
    assert!(
        h.contains("<style"),
        "CSS should be inlined (self-contained)"
    );
    assert!(h.contains("@media print"), "should have print styles");
    assert!(h.contains("<svg"), "should include SVG illustrations");
    for stage in &STAGES {
        assert!(h.contains(stage.title), "html missing {}", stage.title);
        assert!(
            h.contains(stage.goal),
            "html missing goal for {}",
            stage.title
        );
    }
}
