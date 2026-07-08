//! Printable cheat sheet for the beginner (layer-by-layer) method, plus the
//! theory behind each operation.
//!
//! The content mirrors the stages of [`crate::solver::beginner`], so the
//! printed guide and the animated beginner solution teach the same method.
//! [`html`] returns a self-contained, printable HTML document (inline CSS and
//! SVG diagrams); [`markdown`] returns the same content as Markdown.

use std::fmt::Write as _;

/// One stage of the beginner method as it appears on the cheat sheet.
pub struct CheatStage {
    /// Heading, e.g. "1. Bottom cross".
    pub title: &'static str,
    /// What the stage accomplishes.
    pub goal: &'static str,
    /// Facelet grid cells (`0..9`, row-major) to highlight in the illustration.
    pub highlight: &'static [usize],
    /// Named algorithms: `(what it does, standard notation)`.
    pub algorithms: &'static [(&'static str, &'static str)],
    /// Why the algorithms work (the theory for this stage).
    pub theory: &'static str,
}

/// The seven stages of the beginner method, in order.
pub const STAGES: [CheatStage; 7] = [
    CheatStage {
        title: "1. Bottom cross",
        goal: "Make a plus of the bottom colour on the bottom face, with each \
               edge's side sticker matching its neighbouring centre.",
        highlight: &[1, 3, 5, 7],
        algorithms: &[
            ("Drop a correctly-aligned top edge straight down", "F2"),
            ("Fix an edge that is flipped in place", "U' R' F R"),
        ],
        theory: "Centres never move relative to each other, so they define the \
                 finished colour scheme. Each edge belongs between two centres; \
                 aligning the edge's side sticker with its centre before \
                 inserting guarantees the cross is not just a colour match but a \
                 correctly-permuted one.",
    },
    CheatStage {
        title: "2. Bottom corners",
        goal: "Fill the four bottom corners to complete the first layer.",
        highlight: &[0, 2, 6, 8],
        algorithms: &[
            ("Insert a corner sitting above its slot", "R U R'"),
            ("Take a wrongly-oriented corner back out", "R U' R'"),
            (
                "Repeat the trigger until the corner drops in solved",
                "R U R' U'",
            ),
        ],
        theory: "The trigger R U R' is a conjugate: R sets the corner aside, U \
                 repositions it, R' restores the face. Repeating R U R' U' cycles \
                 a single corner through its three orientations, so at most three \
                 repeats orient it without disturbing the finished cross.",
    },
    CheatStage {
        title: "3. Middle-layer edges",
        goal: "Place the four middle-layer edges, completing the first two layers.",
        highlight: &[3, 5],
        algorithms: &[
            (
                "Insert an edge into the slot on your right",
                "U R U' R' U' F' U F",
            ),
            (
                "Insert an edge into the slot on your left",
                "U' L' U L U F U' F'",
            ),
        ],
        theory: "Each insertion is a commutator-like sequence that swaps the \
                 top edge into the target middle slot while returning every \
                 first-layer piece home. Choosing the right- or left-hand version \
                 depends on which way the edge must travel.",
    },
    CheatStage {
        title: "4. Top cross",
        goal: "Orient the last-layer edges so the top face shows a cross.",
        highlight: &[1, 3, 5, 7],
        algorithms: &[("Orient the last-layer edges", "F R U R' U' F'")],
        theory: "Edge orientation is a mod-2 invariant: flips always come in \
                 even numbers, so the top edges form a dot, an L, or a line. The \
                 algorithm flips the edges it touches; applying it (with the \
                 pattern held correctly) walks dot -> L -> line -> cross.",
    },
    CheatStage {
        title: "5. Top face",
        goal: "Orient the last-layer corners so the whole top face is one colour.",
        highlight: &[0, 1, 2, 3, 4, 5, 6, 7, 8],
        algorithms: &[
            ("Sune: rotate three corners clockwise", "R U R' U R U2 R'"),
            (
                "Anti-sune: rotate three corners anticlockwise",
                "R U2 R' U' R U' R'",
            ),
        ],
        theory: "Corner orientation is a mod-3 invariant: the twists always sum \
                 to a multiple of three. Sune twists three corners by one third \
                 each (net zero mod 3), so repeating it from the right angle \
                 orients all four corners.",
    },
    CheatStage {
        title: "6. Top corners",
        goal: "Permute the last-layer corners into their correct positions.",
        highlight: &[0, 2, 6, 8],
        algorithms: &[("Cycle three corners", "R' F R' B2 R F' R' B2 R2")],
        theory: "With the top face oriented, only the last-layer permutation \
                 remains. This algorithm is a pure 3-cycle of corners: it leaves \
                 orientation and everything below untouched, so repeating it (with \
                 U setups) sorts all four corners.",
    },
    CheatStage {
        title: "7. Top edges",
        goal: "Permute the last-layer edges to finish the cube.",
        highlight: &[1, 3, 5, 7],
        algorithms: &[("Cycle three edges", "R U' R U R U R U' R' U' R2")],
        theory: "The final step is a pure 3-cycle of the last-layer edges. \
                 Because corner and edge permutation parities must agree, once \
                 the corners are solved the edges form a solvable 3-cycle, which \
                 this algorithm resolves.",
    },
];

const INTRO: &str = "This guide solves any 3x3 cube layer by layer. Hold the \
    cube so the colour you start the cross with is on the bottom (D) and keep it \
    there the whole solve. Work through the seven stages in order; each builds on \
    the last. Notation: a letter turns that face 90 degrees clockwise (looking at \
    it), a prime (') turns it anticlockwise, and a 2 turns it twice.";

/// Render the cheat sheet as a self-contained, printable HTML document.
#[must_use]
pub fn html() -> String {
    let mut s = String::new();
    s.push_str("<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    s.push_str("<title>3x3 Rubik's Cube Cheat Sheet</title>\n<style>\n");
    s.push_str(CSS);
    s.push_str("\n</style></head><body>\n");
    s.push_str("<h1>3&times;3 Rubik's Cube &mdash; Beginner Method</h1>\n");
    let _ = writeln!(s, "<p class=\"intro\">{INTRO}</p>");

    for stage in &STAGES {
        s.push_str("<section class=\"stage\">\n");
        let _ = writeln!(s, "<h2>{}</h2>", stage.title);
        s.push_str("<div class=\"body\">\n<div class=\"diagram\">");
        s.push_str(&face_svg(stage.highlight));
        s.push_str("</div>\n<div class=\"text\">\n");
        let _ = writeln!(s, "<p class=\"goal\"><b>Goal:</b> {}</p>", stage.goal);
        s.push_str("<table class=\"algs\"><thead><tr><th>Case</th><th>Algorithm</th></tr></thead><tbody>\n");
        for (name, notation) in stage.algorithms {
            let _ = writeln!(
                s,
                "<tr><td>{name}</td><td><code>{notation}</code></td></tr>"
            );
        }
        s.push_str("</tbody></table>\n");
        let _ = writeln!(
            s,
            "<p class=\"theory\"><b>Why it works:</b> {}</p>",
            stage.theory
        );
        s.push_str("</div>\n</div>\n</section>\n");
    }

    s.push_str("</body></html>\n");
    s
}

/// Render the cheat sheet as Markdown.
#[must_use]
pub fn markdown() -> String {
    let mut s = String::new();
    s.push_str("# 3x3 Rubik's Cube — Beginner Method Cheat Sheet\n\n");
    let _ = write!(s, "{INTRO}\n\n");

    for stage in &STAGES {
        let _ = write!(s, "## {}\n\n", stage.title);
        let _ = write!(s, "**Goal:** {}\n\n", stage.goal);
        s.push_str("| Case | Algorithm |\n|------|-----------|\n");
        for (name, notation) in stage.algorithms {
            let _ = writeln!(s, "| {name} | `{notation}` |");
        }
        let _ = write!(s, "\n**Why it works:** {}\n\n", stage.theory);
    }
    s
}

const CSS: &str = "\
body{font-family:system-ui,-apple-system,Segoe UI,Roboto,sans-serif;color:#2d3436;\
max-width:820px;margin:2rem auto;padding:0 1rem;line-height:1.5}\
h1{font-size:1.6rem;border-bottom:3px solid #f4d03f;padding-bottom:.3rem}\
h2{font-size:1.2rem;margin-top:1.6rem}\
.intro{color:#555}\
.stage{break-inside:avoid;border:1px solid #dfe6e9;border-radius:8px;padding:.6rem 1rem;margin:1rem 0}\
.body{display:flex;gap:1rem;align-items:flex-start}\
.diagram{flex:0 0 auto}\
.text{flex:1 1 auto}\
table.algs{border-collapse:collapse;width:100%;margin:.4rem 0}\
.algs th,.algs td{border:1px solid #dfe6e9;padding:.3rem .5rem;text-align:left;font-size:.95rem}\
.algs th{background:#f7f9fa}\
code{background:#f4f6f7;padding:.1rem .35rem;border-radius:4px;font-size:.95rem}\
.theory{color:#444;font-size:.95rem}\
@media print{body{margin:0;max-width:none}.stage{border-color:#999}a{color:inherit}}";

/// A 3x3 face diagram as inline SVG, with `highlight` cells filled.
#[must_use]
fn face_svg(highlight: &[usize]) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        "<svg viewBox=\"0 0 66 66\" width=\"66\" height=\"66\" role=\"img\">"
    );
    for cell in 0..9 {
        let (row, col) = (cell / 3, cell % 3);
        let x = 2 + col * 21;
        let y = 2 + row * 21;
        let fill = if highlight.contains(&cell) {
            "#f4d03f"
        } else {
            "#dfe6e9"
        };
        let _ = write!(
            s,
            "<rect x=\"{x}\" y=\"{y}\" width=\"20\" height=\"20\" rx=\"3\" \
             fill=\"{fill}\" stroke=\"#2d3436\" stroke-width=\"1.5\"/>"
        );
    }
    s.push_str("</svg>");
    s
}
