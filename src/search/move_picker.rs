use crate::movement::{generate_pseudo_legal_moves};
use crate::movement::move_list::MoveList;
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
        tt_move: Option<Move>,
        killer1: Option<Move>,
        killer2: Option<Move>,
    ) -> Self {

        Self {
            stage: Stage::TTMove,
            tt_move,
            killer1,
            killer2,
            moves : MoveList::default(),
            scores: [0; 256],
            index: 0,
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
            moves : MoveList::default(),
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
                    if let Some(mv) = self.tt_move && mv.is_pseudo_legal(&worker.state) {
                        return Some(mv);
                    }
                }
                Stage::GenerateCaptures => {
                    self.index = 0;

                    //Only generate captures
                    generate_pseudo_legal_moves::<true, false>(&worker.state, &mut self.moves);
                    self.score_captures();
                    self.stage = Stage::YieldCaptures;
                }
                Stage::YieldCaptures => {
                    if let Some(mv) = self.pick_next_best() {
                        if Some(mv) == self.tt_move && mv.captured().is_some(){
                            continue; // Skip the tt move if it was a capture
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
                    self.index = 0;

                    // Generate only quiets
                    self.moves.clear();
                    generate_pseudo_legal_moves::<false, true>(&worker.state, &mut self.moves);
                    self.score_quiets(worker);
                    self.stage = Stage::YieldQuiets;
                }
                Stage::YieldQuiets => {
                    if let Some(mv) = self.pick_next_best() {
                        if Some(mv) == self.tt_move && mv.captured().is_none() {
                            continue;
                        }
                        if Some(mv) == self.killer1 || Some(mv) == self.killer2 {
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

    #[inline(always)]
    fn score_captures(&mut self) {
        for i in 0..self.moves.len {
            self.scores[i] = evaluation::mvv_lva(self.moves.moves[i]);
        }
    }

    #[inline(always)]
    fn score_quiets(&mut self, worker: &SearchWorker) {
        let side_idx = worker.state.board.side_to_move as usize;

        for i in 0..self.moves.len {
            let mv = &self.moves.moves[i];
            self.scores[i] = worker.history[side_idx][mv.from() as usize][mv.to() as usize];
        }
    }

    /// Finds and returns the move with the highest score in the remaining moves.
    #[inline(always)]
    fn pick_next_best(&mut self) -> Option<Move> {
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
        Some(mv)
    }
}