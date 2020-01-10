use crate::network::{KSelector, K_MAX, K_MIN};
use std::cmp::{max, min};

pub struct KSelectorZigZag {}

impl Default for KSelectorZigZag {
    fn default() -> Self {
        Self {}
    }
}

impl KSelectorZigZag {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KSelector for KSelectorZigZag {
    fn select_k(
        &mut self,
        current_k: i8,
        rott: u32,
        avg_rott: f64,
        std_rott: f64,
        _prev_rott: u32,
    ) -> i8 {
        if rott as f64 > avg_rott + std_rott {
            max(K_MAX, current_k + 1)
        } else {
            min(K_MIN, current_k - 1)
        }
    }
}
