use super::congestion_detection::CongestionState;

mod sdmi;
mod sdsi;
mod sdsi_exponential_backoff;

pub const K_MAX: i8 = 4;
pub const K_MIN: i8 = 1;

pub use sdmi::KPolicySDMI;
pub use sdsi::KPolicySDSI;
pub use sdsi_exponential_backoff::KPolicySDMIExponentialBackoff;

pub trait KPolicy {
    fn select_k(&mut self, congestion_state: CongestionState, current_k: i8) -> Option<i8>;
}
