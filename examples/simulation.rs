extern crate network_emulator;

use crossbeam::channel::{unbounded, Receiver, Sender};
use csv::Writer;
use network_emulator::{
    common::timestamp, hoip, network, read_channel_configs, ChannelConfig, Network,
};
use serde::Serialize;
use std::{
    error::Error,
    fs::OpenOptions,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Debug, Serialize, Clone, Copy)]
pub enum OP {
    Master,
    Slave,
}

#[derive(Debug, Serialize)]
pub struct Record {
    // Message stats.
    op: OP,
    delay_until_processed: f64,
    k: i8,

    // Channel stats.
    transmission_delay_micros: u64,
    capacity: f64,
    prob_good_to_bad: f64,
    prob_bad_to_good: f64,
    err_rate_good: f64,
    err_rate_bad: f64,

    // Network config.
    adjust_k: bool,
}

fn write_simulation_results(i: usize, rx_record: Receiver<Record>) {
    let out = OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!("simulation_results/{}.csv", i))
        .expect("failed to open simulation results file");
    let mut wtr = Writer::from_writer(out);
    for record in rx_record {
        wtr.serialize(record).expect("failed serialize record");
    }
    wtr.flush().expect("failed to flush writer");
}

pub fn run_network<
    A: 'static + Send + Clone + hoip::Serializable,
    B: 'static + Send + hoip::Serializable,
>(
    mut network: Network<A, B>,
    op: OP,
    running: Arc<AtomicBool>,
    tx_record: Sender<Record>,
    sample_packet: A,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            if let Some((ts, _)) = network.try_recv() {
                let now = timestamp();
                let channel_config = network.channel_config();
                tx_record
                    .send(Record {
                        op,
                        delay_until_processed: (now - ts) as f64 / 1000.0,
                        k: network.k(),
                        transmission_delay_micros: channel_config.transmission_delay_micros,
                        capacity: channel_config.capacity,
                        prob_good_to_bad: channel_config.gilbert_elliot_config.prob_good_to_bad,
                        prob_bad_to_good: channel_config.gilbert_elliot_config.prob_bad_to_good,
                        err_rate_good: channel_config.gilbert_elliot_config.err_rate_good,
                        err_rate_bad: channel_config.gilbert_elliot_config.err_rate_bad,
                        adjust_k: network.adjust_k(),
                    })
                    .expect("failed to send record from slave");
            }
            network.send(sample_packet.clone());
            thread::sleep(std::time::Duration::from_millis(1));
        }
    })
}

fn run_simulation(
    i: usize,
    simulation_time: Duration,
    adjust_k: bool,
    channel_config: &ChannelConfig,
) {
    let (master, slave) = network(channel_config, adjust_k);

    let simulation_running = Arc::new(AtomicBool::new(true));

    let (tx_record, rx_record) = unbounded();

    let writer_thread = thread::spawn(move || {
        write_simulation_results(i, rx_record);
    });

    let running = simulation_running.clone();

    let slave_thread = run_network(
        slave,
        OP::Slave,
        running.clone(),
        tx_record.clone(),
        hoip::PayloadS2M::new([0.0; 3]),
    );
    let master_thread = run_network(
        master,
        OP::Master,
        running,
        tx_record,
        hoip::PayloadM2S::new([0.0; 3], [0.0; 3]),
    );

    thread::sleep(simulation_time);

    simulation_running.store(false, Ordering::SeqCst);

    slave_thread.join().unwrap();
    master_thread.join().unwrap();
    writer_thread.join().unwrap();
}

fn main() -> Result<(), Box<dyn Error>> {
    let simulation_time = Duration::from_secs(10);
    let channel_configs = read_channel_configs("examples/channel_configs.yml")?;

    let channel_configs: Vec<_> = (1000_000..1000001)
        .step_by(500000000)
        .map(|i| {
            let mut channel_config = channel_configs[0].clone();
            channel_config.capacity = (i) as _;
            channel_config
        })
        .collect();

    for (i, channel_config) in channel_configs.iter().enumerate() {
        run_simulation(i * 2, simulation_time, true, channel_config);
        //run_simulation(i * 2 + 1, simulation_time, false, channel_config);
    }
    Ok(())
}
