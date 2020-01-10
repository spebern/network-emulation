use crate::network::{KSelector, K_MAX, K_MIN};
use std::cmp::{max, min};

pub struct KSelectorTrend {
    // The threshold of the trend index.
    s_threshold: f64,
    // Exponential decaying factor that weights the impact of the current `rott` on `s_f`.
    gamma: f64,
    // The current trend index.
    s_f: f64,
}

impl Default for KSelectorTrend {
    fn default() -> Self {
        Self {
            s_threshold: 0.4,
            gamma: 0.9,
            s_f: 0.0,
        }
    }
}

impl KSelectorTrend {
    pub fn new(gamma: f64, s_threshold: f64) -> Self {
        Self {
            s_threshold,
            gamma,
            s_f: 0.0,
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
        self.s_f = (1.0 - self.gamma) * self.s_f;
        if rott > prev_rott {
            self.s_f += self.gamma;
        }
        if self.s_threshold < self.s_f {
            max(K_MAX, current_k + 1)
        } else {
            min(K_MIN, current_k - 1)
        }
    }
}
