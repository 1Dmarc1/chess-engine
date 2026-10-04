use crate::movement::{MoveList, generate_pseudo_legal_moves};
use crate::search::evaluation;
use crate::search::search_worker::SearchWorker;
use crate::types::Move;

enum Stage {
    TTMove,
    GenerateCaptures,
    YieldCaptures,
    Killer1,
    Killer2,
    GenerateQuiets,
    YieldQuiets,
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
    quiescence_only: bool,
}

impl MovePicker {
    pub fn new(
        worker: &SearchWorker,
        tt_move: Option<Move>,
        killer1: Option<Move>,
        killer2: Option<Move>,
    ) -> Self {
        let mut moves = MoveList::default();
        generate_pseudo_legal_moves::<true, true>(&worker.state, &mut moves); // Generate all moves

        let valid_tt = tt_move.filter(|&m| Self::list_contains(&moves, m));
        let valid_k1 = killer1.filter(|&m| Self::list_contains(&moves, m));
        let valid_k2 = killer2.filter(|&m| Self::list_contains(&moves, m));

        Self {
            stage: Stage::TTMove,
            tt_move: valid_tt,
            killer1: valid_k1,
            killer2: valid_k2,
            moves,
            scores: [0; 256],
            index: 0,
            quiescence_only: false,
        }
    }

    /// Initializes the move picker for quiescence search. This will only yield captures and promotions.
    pub fn new_quiescence(worker: &SearchWorker, tt_move: Option<Move>) -> Self {
        let mut moves = MoveList::default();
        generate_pseudo_legal_moves::<true, false>(&worker.state, &mut moves); // In quiescence search we are only interested in captures and promotions
        let valid_tt = tt_move.filter(|&m| Self::list_contains(&moves, m));
        Self {
            stage: Stage::TTMove,
            tt_move: valid_tt,
            killer1: None,
            killer2: None,
            moves,
            scores: [0; 256],
            index: 0,
            quiescence_only: true,
        }
    }

    /// Returns the next best available move or None.
    pub fn next_move(&mut self, worker: &SearchWorker) -> Option<Move> {
        loop {
            match self.stage {
                Stage::TTMove => {
                    self.stage = Stage::GenerateCaptures;
                    if let Some(mv) = self.tt_move {
                        return Some(mv);
                    }
                }
                Stage::GenerateCaptures => {
                    self.index = 0;
                    self.score_captures();
                    self.stage = Stage::YieldCaptures;
                }
                Stage::YieldCaptures => {
                    if let Some(mv) = self.pick_next_best() {
                        if Some(mv) == self.tt_move {
                            continue; // Skip the tt move
                        }
                        return Some(mv);
                    }
                    if self.quiescence_only {
                        self.stage = Stage::Done;
                    } else {
                        self.stage = Stage::Killer1;
                    }
                }
                Stage::Killer1 => {
                    self.stage = Stage::Killer2;
                    if let Some(mv) = self.killer1
                        && Some(mv) != self.tt_move
                    {
                        return Some(mv);
                    }
                }
                Stage::Killer2 => {
                    self.stage = Stage::GenerateQuiets;
                    if let Some(mv) = self.killer2
                        && Some(mv) != self.tt_move
                    {
                        return Some(mv);
                    }
                }
                Stage::GenerateQuiets => {
                    self.index = 0;
                    self.score_quiets(worker);
                    self.stage = Stage::YieldQuiets;
                }
                Stage::YieldQuiets => {
                    if let Some(mv) = self.pick_next_best() {
                        if Some(mv) == self.tt_move {
                            continue;
                        }
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

    fn score_captures(&mut self) {
        for i in 0..self.moves.len {
            let mv = &self.moves.moves[i];
            if mv.captured().is_some() || mv.is_promotion() {
                self.scores[i] = evaluation::mvv_lva(mv);
            } else {
                self.scores[i] = -1_000_000; // Hide quiet moves during capture phase
            }
        }
    }

    fn score_quiets(&mut self, worker: &SearchWorker) {
        let side_idx = worker.state.board.side_to_move as usize;
        for i in 0..self.moves.len {
            let mv = &self.moves.moves[i];
            if Some(*mv) == self.tt_move || Some(*mv) == self.killer1 || Some(*mv) == self.killer2 {
                self.scores[i] = -999_999;
                continue;
            }
            if mv.captured().is_none() && !mv.is_promotion() {
                self.scores[i] = worker.history[side_idx][mv.from() as usize][mv.to() as usize];
            } else {
                self.scores[i] = -1_000_000; // Hide captures during quiet phase
            }
        }
    }

    /// Finds and returns the move with the highest score in the remaining moves.
    fn pick_next_best(&mut self) -> Option<Move> {
        if self.index >= self.moves.len {
            return None;
        }

        let mut best_idx = self.index;
        for i in (self.index + 1)..self.moves.len {
            if self.scores[i] > self.scores[best_idx] {
                best_idx = i;
            }
        }

        // If the best found score is in the penalize zone no more moves exists
        if self.scores[best_idx] < -500_000 {
            return None;
        }

        self.moves.moves.swap(best_idx, self.index);
        self.scores.swap(best_idx, self.index);

        let mv = self.moves.moves[self.index];
        self.index += 1;
        Some(mv)
    }

    /// Returns true if the target move exists in the moves list.
    fn list_contains(moves: &MoveList, target: Move) -> bool {
        for i in 0..moves.len {
            if moves.moves[i] == target {
                return true;
            }
        }
        false
    }
}
