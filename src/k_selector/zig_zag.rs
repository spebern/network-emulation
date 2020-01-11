use crate::network::{KSelector, K_MAX, K_MIN};
use std::cmp::max;

pub struct KSelectorZigZag {
    // The cool off after k has been adapted.
    cooloff: usize,
    // The counter for counting the steps without adaption of k.
    counter: usize,
}

impl Default for KSelectorZigZag {
    fn default() -> Self {
        Self {
            cooloff: 10,
            counter: 0,
        }
    }
}

impl KSelectorZigZag {
    pub fn new(cooloff: usize) -> Self {
        Self {
            cooloff,
            counter: 0,
        }
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
        if self.counter < self.cooloff {
            self.counter += 1;
            return current_k;
        } else {
            self.counter = 0;
        }

        if rott as f64 > avg_rott + std_rott {
            K_MAX
        } else {
            self.counter = 0;
            max(K_MIN, current_k - 1)
        }
    }
}
