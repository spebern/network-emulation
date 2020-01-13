use crate::congestion_detection::{CongestionDetector, CongestionState};

pub struct NetworkAnalyzer<CD> {
    // The estimated average delay.
    avg_rott: f64,
    // The estimated standard deviation of the delay.
    std_rott: f64,
    // The previous rott.
    prev_rott: u32,
    // The congestion detector.
    congestion_detector: CD,
    // The weight of the exponential decaying average used for
    // calculating the average delay and its variance.
    w: f64,
    // The counter for counting ticks between state changes.
    counter: usize,
    // The current congestion state.
    state: CongestionState,
}

impl<CD: CongestionDetector> NetworkAnalyzer<CD> {
    pub fn new(congestion_detector: CD, w: f64) -> Self {
        Self {
            avg_rott: 0.0,
            std_rott: 0.0,
            prev_rott: 0,
            congestion_detector,
            w,
            counter: 0,
            state: CongestionState::NotSure,
        }
    }

    fn calc_avg_and_std_rott(&self, rott: u32) -> (f64, f64) {
        let avg_rott = (1.0 - self.w) * self.avg_rott + self.w * rott as f64;
        let std_rott = (1.0 - 2.0 * self.w) + 2.0 * self.w * (rott as f64 - avg_rott).abs();
        (avg_rott, std_rott)
    }

    pub fn update_state(&mut self, rott: u32) {
        let (avg_rott, std_rott) = self.calc_avg_and_std_rott(rott);

        self.state =
            self.congestion_detector
                .is_congested(rott, avg_rott, std_rott, self.prev_rott);

        self.avg_rott = avg_rott;
        self.std_rott = std_rott;
        self.prev_rott = rott;
    }

    pub fn state(&self) -> CongestionState {
        self.state
    }
}
