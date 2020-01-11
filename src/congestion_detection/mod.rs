mod biaz;
mod trend;
mod window;
mod zig_zag;

pub use biaz::Biaz;
pub use trend::Trend;
pub use window::Window;
pub use zig_zag::ZigZag;

pub enum CongestionState {
    // The algorithm is not sure.
    NotSure,
    // The network is congested.
    Congested,
    // The network is not congested.
    NotCongested,
}

pub trait CongestionDetector {
    fn is_congested(
        &mut self,
        current_k: i8,
        rott: u32,
        avg_rott: f64,
        std_rott: f64,
        prev_rott: u32,
    ) -> CongestionState;
}
