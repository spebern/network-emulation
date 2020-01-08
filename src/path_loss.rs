use async_std::sync::{Arc, Mutex};
use rand::distributions::{Distribution, Uniform};

#[derive(Clone)]
enum GilbertElliotState {
    Good,
    Bad,
}

#[derive(Clone)]
pub struct GilbertElliot {
    prob_good_to_bad: f64,
    prob_bad_to_good: f64,
    err_rate_good: f64,
    err_rate_bad: f64,
    state: Arc<Mutex<GilbertElliotState>>,
    uniform_sampler: Uniform<f64>,
}

impl GilbertElliot {
    pub fn new(
        prob_good_to_bad: f64,
        prob_bad_to_good: f64,
        err_rate_good: f64,
        err_rate_bad: f64,
    ) -> Self {
        let uniform_sampler = Uniform::new(0.0, 1.0);
        let state = Arc::new(Mutex::new(GilbertElliotState::Good));
        Self {
            prob_good_to_bad,
            prob_bad_to_good,
            err_rate_good,
            err_rate_bad,
            state,
            uniform_sampler,
        }
    }

    async fn maybe_next_state(&self) -> GilbertElliotState {
        let uniform_sample = self.uniform_sampler.sample(&mut rand::thread_rng());
        let mut state = self.state.lock().await;
        match *state {
            GilbertElliotState::Good => {
                if uniform_sample < self.prob_good_to_bad {
                    *state = GilbertElliotState::Bad;
                }
            }
            GilbertElliotState::Bad => {
                if uniform_sample < self.prob_bad_to_good {
                    *state = GilbertElliotState::Good;
                }
            }
        };
        state.clone()
    }

    pub async fn transmit(&self) -> bool {
        let state = self.maybe_next_state().await;
        let uniform_sample = self.uniform_sampler.sample(&mut rand::thread_rng());
        match state {
            GilbertElliotState::Good => uniform_sample >= self.err_rate_good,
            GilbertElliotState::Bad => uniform_sample >= self.err_rate_bad,
        }
    }
}
