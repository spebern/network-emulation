use super::{CongestionDetector, CongestionState};

pub struct Window {
    // The number of samples inside of a window.
    n: isize,
    // The previous `N` notification delays.
    previous_rotts: Vec<u32>,
    // A counter that starts after each state reset.
    counter: isize,
    // Number of consecutive increasing delays.
    increasing_rotts_in_a_row: isize,
}

impl Window {
    pub fn new(n: isize) -> Self {
        Self {
            n,
            previous_rotts: (0..n).map(|_| 0).collect(),
            counter: 0,
            increasing_rotts_in_a_row: 0,
        }
    }

    fn reset(&mut self) {
        for rott in self.previous_rotts.iter_mut() {
            *rott = 0;
        }
        self.increasing_rotts_in_a_row = 0;
        self.counter = 0;
    }
}

impl Default for Window {
    fn default() -> Self {
        Window {
            n: 8,
            previous_rotts: vec![0; 8],
            counter: 0,
            increasing_rotts_in_a_row: 0,
        }
    }
}

impl CongestionDetector for Window {
    fn is_congested(
        &mut self,
        current_k: i8,
        rott: u32,
        avg_rott: f64,
        _std_rott: f64,
        _prev_rott: u32,
    ) -> CongestionState {
        if rott as f64 > avg_rott {
            self.increasing_rotts_in_a_row += 1;
        } else {
            self.increasing_rotts_in_a_row = 0;
        }

        self.previous_rotts[(self.counter % self.n) as usize] = rott;

        // count up the rounds after last state change
        self.counter += 1;

        // check if we exceeded the number of increasing delays in a row
        if self.increasing_rotts_in_a_row > self.n {
            self.reset();
            return CongestionState::Congested;
        }

        let sum: f64 = self
            .previous_rotts
            .iter()
            .fold(0.0, |acc, x| acc + *x as f64);
        let avg = sum / self.previous_rotts.len() as f64;
        let mut rott = rott;
        let mut increasing_rotts = 0;
        for i in self.counter + 1..self.counter + self.n + 1 {
            if rott < (0.90 * avg) as _ || rott > (1.10 * avg) as _ {
                return CongestionState::NotSure;
            }
            let i = (i % self.n) as usize;
            let next_rott = self.previous_rotts[i];
            if rott < next_rott {
                increasing_rotts += 1;
            }
            rott = next_rott;
        }

        // if we don't have an increasing or decreasing tend try to increase k
        if increasing_rotts < self.n {
            return CongestionState::NotCongested;
        } else {
            return CongestionState::NotSure;
        }
    }
}
