use serde::Deserialize;
use serde_yaml;
use std::{error::Error, fs::File, path::Path};

#[derive(Debug, Deserialize, Clone)]
pub struct GilbertElliotConfig {
    pub prob_good_to_bad: f64,
    pub prob_bad_to_good: f64,
    pub err_rate_good: f64,
    pub err_rate_bad: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChannelConfig {
    #[serde(default = "default_transmission_delay_micros")]
    pub transmission_delay_micros: u64,
    pub capacity: f64,
    pub gilbert_elliot_config: GilbertElliotConfig,
}

fn default_transmission_delay_micros() -> u64 {
    5_000
}

pub fn read_channel_configs<P: AsRef<Path>>(path: P) -> Result<Vec<ChannelConfig>, Box<dyn Error>> {
    let rdr = File::open(path)?;
    Ok(serde_yaml::from_reader(rdr)?)
}
