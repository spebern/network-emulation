use crate::network::KSelector;

pub struct KSelectorBiaz {}

impl Default for KSelectorBiaz {
    fn default() -> Self {
        Self {}
    }
}

impl KSelectorBiaz {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KSelector for KSelectorBiaz {
    fn select_k(
        &mut self,
        _current_k: i8,
        _rott: u32,
        _avg_rott: f64,
        _std_rott: f64,
        _prev_rott: u32,
    ) -> i8 {
        unimplemented!()
    }
}
