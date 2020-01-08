extern crate network_emulator;

use crossbeam::channel::{unbounded, Receiver, Sender};
use csv::Writer;
use network_emulator::{
    common::timestamp,
    hoip::{PayloadM2S, PayloadS2M, PayloadType, Serializable},
    network_emulator::setup_network_emulator,
    NetworkModule,
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

    // Network module stats.
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

pub fn run_network<A: 'static + Send + Clone + Serializable, B: 'static + Send + Serializable>(
    mut network_module: NetworkModule<A, B>,
    op: OP,
    running: Arc<AtomicBool>,
    tx_record: Sender<Record>,
    sample_packet: A,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            if let Some((ts, _)) = network_module.try_recv() {
                let now = timestamp();
                //println!("dt: {}", (now - ts) as f64 / 1000.0);
                tx_record
                    .send(Record {
                        op,
                        delay_until_processed: (now - ts) as f64 / 1000.0,
                        k: network_module.k(),
                        adjust_k: network_module.adjust_k(),
                    })
                    .expect("failed to send record from slave");
            }
            network_module.send(sample_packet.clone());
            thread::sleep(std::time::Duration::from_millis(1));
        }
    })
}

fn run_simulation(i: usize, simulation_time: Duration) {
    let master = NetworkModule::<PayloadM2S, PayloadS2M>::new(PayloadType::Master, true);
    let slave = NetworkModule::<PayloadS2M, PayloadM2S>::new(PayloadType::Slave, true);

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
        PayloadS2M::new([0.0; 3]),
    );

    let master_thread = run_network(
        master,
        OP::Master,
        running,
        tx_record,
        PayloadM2S::new([0.0; 3], [0.0; 3]),
    );

    thread::sleep(simulation_time);

    simulation_running.store(false, Ordering::SeqCst);

    slave_thread.join().unwrap();
    master_thread.join().unwrap();
    writer_thread.join().unwrap();
}

fn main() -> Result<(), Box<dyn Error>> {
    let simulation_time = Duration::from_secs(10);

    //for (i, rate_kbs) in (500..=1000).step_by(50).enumerate() {
    for (i, rate_kbs) in (400..=400).step_by(50).enumerate() {
        setup_network_emulator(rate_kbs, 3);
        std::thread::sleep(Duration::from_secs(3));
        run_simulation(i, simulation_time);
    }
    Ok(())
}
