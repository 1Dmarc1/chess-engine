use crate::move_gen::generate_pseudo_legal_moves;
use crate::move_gen::move_list::MoveList;
use crate::search::evaluation;
use crate::search::search_worker::core::SearchWorker;
use crate::types::Move;
use std::cmp::PartialEq;

#[derive(Eq, PartialEq)]
enum Stage {
    TTMove,
    GenerateCaptures,
    YieldGoodCaptures,
    Killer1,
    Killer2,
    GenerateQuiets,
    YieldQuiets,
    YieldBadCaptures,
    Done,
}

pub struct MovePicker {
    stage: Stage,
    tt_move: Option<Move>,
    killer1: Option<Move>,
    killer2: Option<Move>,
    moves: MoveList,
    scores: [i32; 256],
    index: usize,
    captures_end: usize,       // The first index which is not a capture move
    bad_captures_start: usize, // The first index in the moves list which is a bad capture
    quiescence_only: bool,
    ply : usize,
}

impl MovePicker {
    pub fn new(tt_move: Option<Move>, killer1: Option<Move>, killer2: Option<Move>, ply : usize) -> Self {
        Self {
            stage: Stage::TTMove,
            tt_move,
            killer1,
            killer2,
            moves: MoveList::default(),
            scores: [0; 256],
            index: 0,
            captures_end: 0,
            bad_captures_start: 0,
            quiescence_only: false,
            ply
        }
    }

    /// Initializes the move picker for quiescence search. This will only yield captures and promotions.
    pub fn new_quiescence(tt_move: Option<Move>) -> Self {
        Self {
            stage: Stage::TTMove,
            tt_move,
            killer1: None,
            killer2: None,
            moves: MoveList::default(),
            scores: [0; 256],
            index: 0,
            captures_end: 0,
            bad_captures_start: 0,
            quiescence_only: true,
            ply : 0,
        }
    }

    /// Returns the next best available move or None.
    pub fn next_move(&mut self, worker: &SearchWorker) -> Option<Move> {
        loop {
            match self.stage {
                Stage::TTMove => {
                    if let Some(mv) = self.yield_tt_move(worker) {
                        return Some(mv);
                    }
                }
                Stage::GenerateCaptures => self.generate_captures(worker),
                Stage::YieldGoodCaptures => {
                    if let Some(mv) = self.yield_good_captures() {
                        return Some(mv);
                    }
                }
                Stage::Killer1 => {
                    self.stage = Stage::Killer2;
                    if let Some(mv) = self.yield_killer(self.killer1, worker) {
                        return Some(mv);
                    }
                }
                Stage::Killer2 => {
                    self.stage = Stage::GenerateQuiets;
                    if let Some(mv) = self.yield_killer(self.killer2, worker) {
                        return Some(mv);
                    }
                }
                Stage::GenerateQuiets => self.generate_quiets(worker),
                Stage::YieldQuiets => {
                    if let Some(mv) = self.yield_quiets() {
                        return Some(mv);
                    }
                }
                Stage::YieldBadCaptures => {
                    if let Some(mv) = self.yield_bad_captures() {
                        return Some(mv);
                    }
                }
                Stage::Done => {
                    return None;
                }
            }
        }
    }

    #[inline]
    fn yield_tt_move(&mut self, search_worker: &SearchWorker) -> Option<Move> {
        self.stage = Stage::GenerateCaptures;
        if let Some(mv) = self.tt_move
            && mv.is_pseudo_legal(&search_worker.state)
        {
            return Some(mv);
        }
        None
    }

    #[inline]
    fn yield_killer(&mut self, killer: Option<Move>, worker: &SearchWorker) -> Option<Move> {
        if let Some(mv) = killer {
            if Some(mv) != self.tt_move
                && mv.captured().is_none()
                && mv.is_pseudo_legal(&worker.state)
            {
                return Some(mv);
            }
        }
        None
    }

    #[inline]
    fn generate_captures(&mut self, search_worker: &SearchWorker) {
        self.index = 0;
        generate_pseudo_legal_moves::<true, false>(&search_worker.state, &mut self.moves);
        self.score_captures(search_worker);
        self.captures_end = self.moves.len;
        self.stage = Stage::YieldGoodCaptures;
    }

    #[inline]
    fn yield_good_captures(&mut self) -> Option<Move> {
        if let Some((mv, score)) = self.pick_next_best() {
            if score < 0 {
                self.bad_captures_start = self.index - 1;
                self.stage = if self.quiescence_only {
                    Stage::Done
                } else {
                    Stage::Killer1
                };
                return None;
            }
            if Some(mv) == self.tt_move && mv.captured().is_some() {
                return None; // Skip already evaluated TT capture
            }
            return Some(mv);
        }
        self.bad_captures_start = self.index;
        self.stage = if self.quiescence_only {
            Stage::Done
        } else {
            Stage::Killer1
        };
        None
    }

    #[inline]
    fn generate_quiets(&mut self, search_worker: &SearchWorker) {
        self.index = self.moves.len;
        generate_pseudo_legal_moves::<false, true>(&search_worker.state, &mut self.moves); // Append the quiet moves to the list
        self.score_quiets(search_worker);
        self.stage = Stage::YieldQuiets;
    }

    #[inline]
    fn yield_quiets(&mut self) -> Option<Move> {
        if let Some((mv, _)) = self.pick_next_best() {
            if Some(mv) == self.tt_move || Some(mv) == self.killer1 || Some(mv) == self.killer2 {
                return None;
            }
            return Some(mv);
        }
        // Setup for Bad Captures once quiets are exhausted
        self.moves.len = self.captures_end;
        self.index = self.bad_captures_start;
        self.stage = Stage::YieldBadCaptures;
        None
    }

    #[inline]
    fn yield_bad_captures(&mut self) -> Option<Move> {
        if let Some((mv, _)) = self.pick_next_best() {
            if Some(mv) == self.tt_move {
                return None;
            }
            return Some(mv);
        }
        self.stage = Stage::Done;
        None
    }

    #[inline]
    fn score_captures(&mut self, worker: &SearchWorker) {
        for i in 0..self.moves.len {
            let mv = self.moves.moves[i];

            let score = worker.history.score_capture(mv);
            let is_good = if Self::is_promising_capture(mv) {
                true
            } else {
                worker.state.is_move_greater_equal(mv, 0)
            };
            const GOOD: i32 = 10_000_000;
            self.scores[i] = if is_good {
                score + GOOD
            } else {
                score - GOOD
            };
        }
    }

    #[inline]
    fn score_quiets(&mut self, worker: &SearchWorker) {
        let side_idx = worker.state.board.side_to_move as usize;

        for i in self.index..self.moves.len {
            let mv = self.moves.moves[i];
            self.scores[i] = worker.history.score_quiet_move(mv, self.ply, &worker.stack);
        }
    }

    #[inline]
    fn pick_next_best(&mut self) -> Option<(Move, i32)> {
        if self.index >= self.moves.len {
            return None;
        }

        let mut best_idx = self.index;
        let mut best_score = self.scores[best_idx];

        for i in (self.index + 1)..self.moves.len {
            if self.scores[i] > best_score {
                best_score = self.scores[i];
                best_idx = i;
            }
        }

        self.moves.moves.swap(best_idx, self.index);
        self.scores.swap(best_idx, self.index);

        let mv = self.moves.moves[self.index];
        self.index += 1;
        Some((mv, best_score))
    }

    #[inline]
    fn is_promising_capture(mv: Move) -> bool {
        let victim_val = evaluation::get_piece_score(mv.captured().unwrap_or(0) as usize);
        let attacker_val = evaluation::get_piece_score(mv.piece_type() as usize);
        victim_val >= attacker_val
    }
}
