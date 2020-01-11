use super::{CongestionDetector, CongestionState};

pub struct Biaz {}

impl Default for Biaz {
    fn default() -> Self {
        Self {}
    }
}

impl Biaz {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CongestionDetector for Biaz {
    fn is_congested(
        &mut self,
        _current_k: i8,
        _rott: u32,
        _avg_rott: f64,
        _std_rott: f64,
        _prev_rott: u32,
    ) -> CongestionState {
        unimplemented!()
    }
}
