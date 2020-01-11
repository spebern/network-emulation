use super::{CongestionDetector, CongestionState};

pub struct Trend {
    // The threshold of the trend index.
    s_threshold: f64,
    // Exponential decaying factor that weights the impact of the current `rott` on `s_f`.
    gamma: f64,
    // The current trend index.
    s_f: f64,
}

impl Default for Trend {
    fn default() -> Self {
        Self {
            s_threshold: 0.4,
            gamma: 0.9,
            s_f: 0.0,
        }
    }
}

impl Trend {
    pub fn new(gamma: f64, s_threshold: f64) -> Self {
        Self {
            s_threshold,
            gamma,
            s_f: 0.0,
        }
    }
}

impl CongestionDetector for Trend {
    fn is_congested(
        &mut self,
        _current_k: i8,
        rott: u32,
        _avg_rott: f64,
        _std_rott: f64,
        prev_rott: u32,
    ) -> CongestionState {
        self.s_f = (1.0 - self.gamma) * self.s_f;
        if rott > prev_rott {
            self.s_f += self.gamma;
        }
        if self.s_threshold < self.s_f {
            CongestionState::Congested
        } else {
            CongestionState::NotCongested
        }
    }
}
