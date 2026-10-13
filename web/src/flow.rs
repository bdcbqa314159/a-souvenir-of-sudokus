//! The decisions the board makes, as plain functions.
//!
//! They used to live inside the `App` component, which is the one place
//! `cargo test` cannot reach: a guard could be dropped in a refactor and
//! nothing would fail. Each of these answers one question with no signals, no
//! clock and no DOM, so the component is left holding only the wiring.

use crate::i18n::Mode;

/// What pressing a mode in the picker should do.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ModeStep {
    /// already there — a press on the mode you are in changes nothing
    Stay,
    /// the phantoms finished this grid; nothing switches until a new one
    RefuseTaken,
    /// hangman interrogates digits as they land, and this grid has none left
    RefuseSolved,
    /// leaving the phantom's room asks first: this press is the question
    Confirm,
    /// go
    Enter(Mode),
}

/// Leaving the phantom mid-pact throws the grid away, so it takes two presses
/// — but only the second within this window counts as an answer to the first.
pub const LEAVE_WINDOW_MS: f64 = 3_000.0;

pub fn next_mode(
    current: Mode,
    target: Mode,
    phantom_took_it: bool,
    target_grid_solved: bool,
    leave_armed_at: Option<f64>,
    now: f64,
) -> ModeStep {
    if current == target {
        return ModeStep::Stay;
    }
    if phantom_took_it {
        return ModeStep::RefuseTaken;
    }
    if target == Mode::Hangman && target_grid_solved {
        return ModeStep::RefuseSolved;
    }
    if current == Mode::Phantom {
        let answered = leave_armed_at.is_some_and(|t| now - t < LEAVE_WINDOW_MS);
        if !answered {
            return ModeStep::Confirm;
        }
    }
    ModeStep::Enter(target)
}

/// Which square a command filled, by comparing the board before and after.
/// The engine's reply carries the index, but the frontend's `play` answers
/// whether a command worked rather than where — and a diff cannot disagree
/// with what is actually on the board. `None` when nothing moved.
pub fn changed_cell(before: &[i64], after: &[i64]) -> Option<usize> {
    if before.len() != after.len() {
        return None;
    }
    (0..before.len()).find(|&i| before[i] != after[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: f64 = 10_000.0;

    #[test]
    fn pressing_the_mode_you_are_in_does_nothing() {
        for m in [Mode::Classic, Mode::Phantom, Mode::Hangman] {
            assert_eq!(next_mode(m, m, false, false, None, NOW), ModeStep::Stay);
        }
    }

    #[test]
    fn a_grid_the_phantoms_took_switches_to_nothing() {
        for target in [Mode::Classic, Mode::Hangman] {
            assert_eq!(
                next_mode(Mode::Phantom, target, true, false, None, NOW),
                ModeStep::RefuseTaken,
            );
        }
    }

    #[test]
    fn hangman_refuses_a_finished_grid_but_classic_does_not() {
        assert_eq!(
            next_mode(Mode::Classic, Mode::Hangman, false, true, None, NOW),
            ModeStep::RefuseSolved,
        );
        // leaving a finished grid alone is always allowed
        assert_eq!(
            next_mode(Mode::Hangman, Mode::Classic, false, true, None, NOW),
            ModeStep::Enter(Mode::Classic),
        );
    }

    #[test]
    fn leaving_the_phantom_asks_first_then_goes() {
        // the first press is the question...
        assert_eq!(
            next_mode(Mode::Phantom, Mode::Classic, false, false, None, NOW),
            ModeStep::Confirm,
        );
        // ...a second one, soon enough, is the answer
        assert_eq!(
            next_mode(
                Mode::Phantom,
                Mode::Classic,
                false,
                false,
                Some(NOW - 1_000.0),
                NOW
            ),
            ModeStep::Enter(Mode::Classic),
        );
        // and a stale one is a new question, not an answer to the old
        assert_eq!(
            next_mode(
                Mode::Phantom,
                Mode::Classic,
                false,
                false,
                Some(NOW - LEAVE_WINDOW_MS - 1.0),
                NOW
            ),
            ModeStep::Confirm,
        );
    }

    #[test]
    fn the_confirm_guards_the_way_to_hangman_too() {
        // phantom -> hangman is a path the two old toggles could not express,
        // and it must not become a way around the question
        assert_eq!(
            next_mode(Mode::Phantom, Mode::Hangman, false, false, None, NOW),
            ModeStep::Confirm,
        );
        assert_eq!(
            next_mode(
                Mode::Phantom,
                Mode::Hangman,
                false,
                false,
                Some(NOW - 100.0),
                NOW
            ),
            ModeStep::Enter(Mode::Hangman),
        );
    }

    #[test]
    fn entering_a_mode_from_classic_needs_no_question() {
        for target in [Mode::Phantom, Mode::Hangman] {
            assert_eq!(
                next_mode(Mode::Classic, target, false, false, None, NOW),
                ModeStep::Enter(target),
            );
        }
    }

    #[test]
    fn changed_cell_finds_the_square_that_moved() {
        let before = vec![0; 81];
        let mut after = before.clone();
        after[40] = 7;
        assert_eq!(changed_cell(&before, &after), Some(40));
        assert_eq!(changed_cell(&before, &before), None);
        // the first difference is the answer: a hint fills exactly one square
        let mut two = before.clone();
        two[3] = 1;
        two[9] = 2;
        assert_eq!(changed_cell(&before, &two), Some(3));
        // a malformed board (the saved game is editable) is not a landing
        assert_eq!(changed_cell(&before, &[1, 2, 3]), None);
    }
}
