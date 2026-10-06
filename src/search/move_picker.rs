use std::cmp::PartialEq;
use crate::movement::generate_pseudo_legal_moves;
use crate::movement::move_list::MoveList;
use crate::search::evaluation;
use crate::search::search_worker::SearchWorker;
use crate::types::Move;

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
    captures_end: usize, // The first index which is not a capture move
    bad_captures_start: usize, // The first index in the moves list which is a bad capture
    quiescence_only: bool,
}


impl MovePicker {
    pub fn new(tt_move: Option<Move>, killer1: Option<Move>, killer2: Option<Move>) -> Self {
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
        }
    }

    /// Returns the next best available move or None.
    pub fn next_move(&mut self, worker: &SearchWorker) -> Option<Move> {
        loop {
            match self.stage {
                Stage::TTMove => {
                    self.stage = Stage::GenerateCaptures;
                    if let Some(mv) = self.tt_move
                        && mv.is_pseudo_legal(&worker.state)
                    {
                        return Some(mv);
                    }
                }
                Stage::GenerateCaptures => {
                    self.index = 0;

                    //Only generate captures
                    generate_pseudo_legal_moves::<true, false>(&worker.state, &mut self.moves);
                    self.score_captures(worker);
                    self.captures_end = self.moves.len;
                    self.stage = Stage::YieldGoodCaptures;
                }
                Stage::YieldGoodCaptures => {
                    if let Some((mv, score)) = self.pick_next_best() {
                        if score < 0 {
                            self.bad_captures_start = self.index - 1;
                            if self.quiescence_only {
                                self.stage = Stage::Done;
                                continue;
                            } else {
                                self.stage = Stage::Killer1;
                                continue;
                            }
                        }
                        if Some(mv) == self.tt_move && mv.captured().is_some() {
                            continue; // Skip the tt move if it was a capture
                        }
                        return Some(mv);
                    }
                    if self.quiescence_only {
                        self.stage = Stage::Done;
                    } else {
                        self.stage = Stage::Killer1;
                    }
                    self.bad_captures_start = self.index;
                }
                Stage::Killer1 => {
                        self.stage = Stage::Killer2;
                        if let Some(mv) = self.killer1
                            && Some(mv) != self.tt_move
                            && mv.captured().is_none()
                            && mv.is_pseudo_legal(&worker.state)
                        {
                            return Some(mv);
                        }
                    }
                    Stage::Killer2 => {
                        self.stage = Stage::GenerateQuiets;
                        if let Some(mv) = self.killer2
                            && Some(mv) != self.tt_move
                            && mv.captured().is_none()
                            && mv.is_pseudo_legal(&worker.state)
                        {
                            return Some(mv);
                        }
                    }
                    Stage::GenerateQuiets => {
                        self.index = self.moves.len;
                        generate_pseudo_legal_moves::<false, true>(&worker.state, &mut self.moves); // Append the quiet moves to the list
                        self.score_quiets(worker);
                        self.stage = Stage::YieldQuiets;
                    }
                    Stage::YieldQuiets => {
                        if let Some((mv, _)) = self.pick_next_best() {
                            if Some(mv) == self.tt_move && mv.captured().is_none() {
                                continue;
                            }
                            if Some(mv) == self.killer1 || Some(mv) == self.killer2 {
                                continue;
                            }
                            return Some(mv);
                        }
                        self.moves.len = self.captures_end; // Hide the quiet moves
                        self.index = self.bad_captures_start;

                        self.stage = Stage::YieldBadCaptures;
                    }
                    Stage::YieldBadCaptures => {
                        if let Some((mv, _)) = self.pick_next_best() {
                            if Some(mv) == self.tt_move { continue; }
                            return Some(mv);
                        }
                        self.stage = Stage::Done;
                    }
                    Stage::Done => {
                        return None;
                    }
                }
            }
        }

    #[inline(always)]
    fn score_captures(&mut self, worker: &SearchWorker) {
        for i in 0..self.moves.len {
            let mv = self.moves.moves[i];

            let mvv_lva = evaluation::mvv_lva(mv);
            let is_good = if mvv_lva >= 0 {
                true
            } else {
                worker.state.is_move_greater_equal(mv, 0)
            };

            const GOOD: i32 = 1_000_000;
            self.scores[i] = if is_good { mvv_lva + GOOD } else { mvv_lva - GOOD };
        }
    }

    #[inline(always)]
    fn score_quiets(&mut self, worker: &SearchWorker) {
        let side_idx = worker.state.board.side_to_move as usize;

        for i in self.index..self.moves.len {
            let mv = &self.moves.moves[i];
            self.scores[i] = worker.history[side_idx][mv.from() as usize][mv.to() as usize];
        }
    }

    /// Finds and returns the move with the highest score in the remaining moves.
    #[inline(always)]
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
}
