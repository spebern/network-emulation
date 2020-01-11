use crate::network::{KSelector, K_MAX, K_MIN};
use std::cmp::{max, min};

pub struct KSelectorTrend {
    // The threshold of the trend index.
    s_threshold: f64,
    // Exponential decaying factor that weights the impact of the current `rott` on `s_f`.
    gamma: f64,
    // The current trend index.
    s_f: f64,
    // The cool off after k has been adapted.
    cooloff: usize,
    // The counter for counting the steps without adaption of k.
    counter: usize,
}

impl Default for KSelectorTrend {
    fn default() -> Self {
        Self {
            s_threshold: 0.4,
            gamma: 0.9,
            s_f: 0.0,
            cooloff: 10,
            counter: 0,
        }
    }
}

impl KSelectorTrend {
    pub fn new(gamma: f64, s_threshold: f64, cooloff: usize) -> Self {
        Self {
            s_threshold,
            gamma,
            s_f: 0.0,
            cooloff,
            counter: 0,
        }
    }
}

impl KSelector for KSelectorTrend {
    fn select_k(
        &mut self,
        current_k: i8,
        rott: u32,
        _avg_rott: f64,
        _std_rott: f64,
        prev_rott: u32,
    ) -> i8 {
        if self.counter < self.cooloff {
            self.counter += 1;
            return current_k;
        } else {
            self.counter = 0;
        }

        self.s_f = (1.0 - self.gamma) * self.s_f;
        if rott > prev_rott {
            self.s_f += self.gamma;
        }
        if self.s_threshold < self.s_f {
            K_MAX
        } else {
            min(K_MIN, current_k - 1)
        }
    }
}
