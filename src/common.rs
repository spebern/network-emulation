use lazy_static::lazy_static;
use std::time::Instant;

lazy_static! {
    static ref INITIAL_TS: Instant = Instant::now();
}

pub fn timestamp() -> u64 {
    let now = Instant::now();
    if now < *INITIAL_TS {
        0
    } else {
        (now - *INITIAL_TS).as_micros() as _
    }
}
