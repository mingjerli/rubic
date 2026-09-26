//! The cube being entered while Editing: painted stickers, the Brush, and its
//! Completion (cached, since analyzing a partial cube runs a search).

use rubic_core::{Completion, Face, Facelets, PartialFacelets};

#[derive(Clone, Debug)]
pub struct Entry {
    partial: PartialFacelets,
    brush: Face,
    completion: Completion,
}

impl Entry {
    /// Only the centers known: painting by hand from scratch.
    #[must_use]
    pub fn blank() -> Self {
        Self::from_partial(PartialFacelets::new())
    }

    /// Every sticker known, from a full cube (the CLI start state, or Edit
    /// from Solve).
    #[must_use]
    pub fn seeded(cube: &Facelets) -> Self {
        Self::from_partial(PartialFacelets::from_facelets(cube))
    }

    /// Start from `partial` (e.g. a finished Scan).
    #[must_use]
    pub fn from_partial(partial: PartialFacelets) -> Self {
        let completion = partial.analyze();
        Self {
            partial,
            brush: Face::U,
            completion,
        }
    }

    /// Paint facelet `i` with the Brush. Centers are locked.
    pub fn paint(&mut self, i: usize) {
        if i % 9 != 4 {
            self.set_partial(self.partial.set(i, self.brush));
        }
    }

    pub fn select(&mut self, face: Face) {
        self.brush = face;
    }

    /// Clear every non-center sticker back to unknown.
    pub fn clear(&mut self) {
        self.set_partial(PartialFacelets::new());
    }

    fn set_partial(&mut self, partial: PartialFacelets) {
        self.completion = partial.analyze();
        self.partial = partial;
    }

    #[must_use]
    pub fn partial(&self) -> &PartialFacelets {
        &self.partial
    }

    #[must_use]
    pub fn brush(&self) -> Face {
        self.brush
    }

    #[must_use]
    pub fn completion(&self) -> &Completion {
        &self.completion
    }

    /// The cube the stickers determine, when they determine exactly one.
    #[must_use]
    pub fn ready_cube(&self) -> Option<Facelets> {
        match &self.completion {
            Completion::Unique(state) => Some(state.to_facelets()),
            _ => None,
        }
    }

    /// Ready: the entered stickers determine exactly one cube.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready_cube().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_is_not_ready() {
        let e = Entry::blank();
        assert!(matches!(e.completion(), Completion::NeedMore { .. }));
        assert!(!e.is_ready());
    }

    #[test]
    fn painting_a_solved_cube_makes_it_ready() {
        let mut e = Entry::blank();
        for i in 0..54 {
            e.select(Face::ALL[i / 9]);
            e.paint(i);
        }
        assert_eq!(e.ready_cube(), Some(Facelets::SOLVED));
    }

    #[test]
    fn centers_are_locked() {
        let mut e = Entry::blank();
        e.select(Face::R);
        e.paint(4); // U center
        assert_eq!(e.partial().get(4), Some(Face::U));
    }

    #[test]
    fn select_changes_the_brush() {
        let mut e = Entry::blank();
        e.select(Face::B);
        assert_eq!(e.brush(), Face::B);
    }

    #[test]
    fn completion_follows_every_edit() {
        let mut e = Entry::seeded(&Facelets::SOLVED);
        assert!(e.is_ready());
        e.clear();
        assert_eq!(e.partial().known_count(), 0);
        assert!(!e.is_ready(), "clearing must refresh the cached Completion");
        e.select(Face::U);
        e.paint(0);
        assert!(matches!(e.completion(), Completion::NeedMore { known: 1 }));
    }
}
