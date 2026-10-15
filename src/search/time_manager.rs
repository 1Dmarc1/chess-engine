use crate::types::Move;
use std::time::{Duration, Instant};
use crate::globals;

const STABILITY_SCALE: [f64; 5] = [1.5, 1.2, 1.0, 0.9, 0.8];

pub struct TimeManager {
    limits: SearchLimits,
    stable: usize,
    prev_best: Option<Move>,
    prev_score: i32,
}

impl TimeManager {
    pub fn new(limits: SearchLimits) -> TimeManager {
        Self {
            limits,
            stable: 0,
            prev_best: None,
            prev_score: 0,
        }
    }

    pub fn update_best_move(&mut self, mv: Move, score: i32) {
        if Some(mv) != self.prev_best {
            self.prev_best = Some(mv);
            self.stable = 0;
        } else {
            self.stable += 1;
        }
        self.prev_score = score;
    }

    pub fn should_continue(&self) -> bool {
        let Some(soft) = self.limits.soft else { return true };
        let scale = STABILITY_SCALE[self.stable.min(STABILITY_SCALE.len() - 1)];
        let budget = soft.mul_f64(scale).min(self.limits.hard.unwrap_or(soft));
        self.limits.start.elapsed() < budget
    }
}

#[derive(Clone, Copy)]
pub struct SearchLimits {
    pub start: Instant,
    pub soft: Option<Duration>,
    pub hard: Option<Duration>,
    pub max_nodes: Option<u64>,
    pub max_depth: i32,
}

impl SearchLimits {

    pub fn from_depth(max_depth : i32) -> Self{
        Self{
            start: Instant::now(),
            soft : None,
            hard : None,
            max_nodes : None,
            max_depth,
        }
    }

    pub fn from_clock(time_left: Duration, inc: Duration, moves_to_go: Option<u32>, overhead: Duration, max_depth : i32) -> Self {
        let usable = time_left.saturating_sub(overhead);
        let mtg = moves_to_go.unwrap_or(25).clamp(1, 50);
        let base = (usable / mtg) + (inc * 3 / 4);

        let hard = (base * 3).min(usable * 4 / 5);
        let soft = base.mul_f64(0.7).min(hard);
        Self {
            start: Instant::now(),
            soft: Some(soft),
            hard: Some(hard),
            max_nodes: None,
            max_depth,
        }
    }

    pub fn from_movetime(t: Duration, overhead: Duration, max_depth : i32) -> Self {
        let t = t.saturating_sub(overhead);
        Self {
            start: Instant::now(),
            soft: None,
            hard: Some(t),
            max_nodes: None,
            max_depth,
        }
    }

    pub fn infinite() -> Self {
        Self {
            start: Instant::now(),
            soft: None,
            hard: None,
            max_nodes: None,
            max_depth: globals::MAX_SEARCH_PLY as i32,
        }
    }

    /// Returns true if the soft time limit has expired.
    pub fn soft_expired(&self) -> bool{
        if let Some(limit) = self.soft{
            return self.start.elapsed() > limit;
        }
        false
    }

    /// Returns true if the hard time limit has expired.
    pub fn hard_expired(&self) -> bool {
        if let Some(limit) = self.hard{
            return self.start.elapsed() > limit;
        }
        false
    }
}
