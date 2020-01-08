mod channel;
pub mod common;
mod config;
pub mod hoip;
mod network;
mod network2;
pub mod network_emulator;
mod path_loss;

pub use config::{read_channel_configs, ChannelConfig, GilbertElliotConfig};
pub use network::Network;
pub use network2::NetworkModule;

use channel::channel;
use hoip::{PayloadType, Serializable};
use path_loss::GilbertElliot;
use std::time::Duration;

pub fn network<A: 'static + Send + Serializable, B: 'static + Send + Serializable>(
    channel_config: &ChannelConfig,
    adjust_k: bool,
) -> (Network<A, B>, Network<B, A>) {
    let path_loss_model = GilbertElliot::new(
        channel_config.gilbert_elliot_config.prob_good_to_bad,
        channel_config.gilbert_elliot_config.prob_bad_to_good,
        channel_config.gilbert_elliot_config.err_rate_good,
        channel_config.gilbert_elliot_config.err_rate_bad,
    );

    let (tx_s2m, rx_s2m) = channel(
        path_loss_model.clone(),
        Duration::from_micros(channel_config.transmission_delay_micros),
        channel_config.capacity,
    );

    let (tx_m2s, rx_m2s) = channel(
        path_loss_model.clone(),
        Duration::from_micros(channel_config.transmission_delay_micros),
        channel_config.capacity,
    );

    let N = 8;
    let w = 0.9;

    let master = Network::new(
        tx_m2s,
        rx_s2m,
        w,
        N,
        channel_config.clone(),
        adjust_k,
        PayloadType::Master,
    );
    let slave = Network::new(
        tx_s2m,
        rx_m2s,
        w,
        N,
        channel_config.clone(),
        adjust_k,
        PayloadType::Slave,
    );

    (master, slave)
}
