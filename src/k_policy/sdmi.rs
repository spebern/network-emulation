use super::{CongestionState, KPolicy, K_MAX, K_MIN};
use std::cmp::max;

pub struct KPolicySDMI;
impl KPolicy for KPolicySDMI {
    fn select_k(&mut self, congestion_state: CongestionState, current_k: i8) -> Option<i8> {
        match congestion_state {
            CongestionState::NotSure => None,
            CongestionState::Congested => Some(K_MAX),
            CongestionState::NotCongested => Some(max(K_MIN, current_k - 1)),
        }
    }
}
