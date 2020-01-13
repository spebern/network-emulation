use super::{CongestionState, KPolicy, K_MAX, K_MIN};
use std::cmp::{max, min};

pub struct KPolicySDSI;
impl KPolicy for KPolicySDSI {
    fn select_k(&mut self, congestion_state: CongestionState, current_k: i8) -> Option<i8> {
        match congestion_state {
            CongestionState::NotSure => None,
            CongestionState::Congested => Some(min(K_MAX, current_k + 1)),
            CongestionState::NotCongested => Some(max(K_MIN, current_k - 1)),
        }
    }
}
