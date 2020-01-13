use super::{CongestionDetector, CongestionState};

pub struct ZigZag {}

impl Default for ZigZag {
    fn default() -> Self {
        Self {}
    }
}

impl ZigZag {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CongestionDetector for ZigZag {
    fn is_congested(
        &mut self,
        rott: u32,
        avg_rott: f64,
        std_rott: f64,
        _prev_rott: u32,
    ) -> CongestionState {
        if rott as f64 > avg_rott + std_rott {
            CongestionState::Congested
        } else {
            CongestionState::NotCongested
        }
    }
}
