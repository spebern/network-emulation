use super::{CongestionState, KPolicy, K_MAX, K_MIN};
use std::cmp::{max, min};

pub struct KPolicySDMIExponentialBackoff {
    zig_zag_counter: usize,
    max_backoff: usize,
    k_over_limit: i8,
    counter: usize,
    congested_in_a_row: usize,
}

impl KPolicySDMIExponentialBackoff {
    pub fn new(max_backoff: usize) -> Self {
        Self {
            zig_zag_counter: 0,
            max_backoff,
            k_over_limit: K_MAX,
            counter: 0,
            congested_in_a_row: 0,
        }
    }

    fn backoff(&self) -> bool {
        self.counter
            > min(
                self.max_backoff,
                1.5_f64.powf(self.zig_zag_counter as f64) as usize,
            )
    }
}

impl KPolicy for KPolicySDMIExponentialBackoff {
    fn select_k(&mut self, congestion_state: CongestionState, current_k: i8) -> Option<i8> {
        self.counter += 1;
        //let cool_off = 40;
        //if self.counter< cool_off {
        //return None;
        //}
        match congestion_state {
            CongestionState::NotSure => return None,
            CongestionState::Congested => {
                self.congested_in_a_row += 1;
                if self.congested_in_a_row > 0 {
                    self.congested_in_a_row = 0;
                    self.counter = 0;
                    Some(K_MAX)
                } else {
                    None
                }
            }
            CongestionState::NotCongested => {
                self.congested_in_a_row = 0;
                self.counter = 0;
                Some(max(K_MIN, current_k - 1))
            }
        }
    }
}
