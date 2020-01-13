use std::time::Instant;

/// A rate limiter.
pub struct RateLimiter {
    previous: Instant,
    rate: f64,
    tokens: f64,
}

impl RateLimiter {
    /// Creates a new limiter with a rate limit of `rate` [Hz].
    pub fn new(rate: f64) -> Self {
        Self {
            previous: Instant::now(),
            tokens: rate,
            rate: rate,
        }
    }

    /// Returns true if surpassed the rate limit.
    pub fn limited(&mut self) -> bool {
        let now = Instant::now();
        let elapsed_us = (now - self.previous).as_micros();
        self.previous = now;

        self.tokens += elapsed_us as f64 * 1.0e-6 * self.rate;
        if self.tokens > self.rate {
            self.tokens = self.rate;
        }

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            false
        } else {
            true
        }
    }

    /// Sets the rate of the limiter in [Hz].
    pub fn set_rate(&mut self, rate: f64) {
        self.rate = rate;
    }

    /// Returns the rate of the limiter in [Hz].
    pub fn rate(&self) -> f64 {
        self.rate
    }
}
