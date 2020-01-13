extern crate network_emulator;

use crossbeam::channel::{unbounded, Receiver, Sender};
use csv::Writer;
use network_emulator::{
    congestion_detection::{self, CongestionDetector},
    hoip::{PayloadM2S, PayloadS2M, PayloadType, Serializable},
    k_policy::{KPolicy, KPolicySDMI, KPolicySDMIExponentialBackoff, KPolicySDSI},
    now, setup_network_emulator, NetworkModule,
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
}

fn write_simulation_results(i: usize, rx_record: Receiver<Record>) {
    let out = OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!("simulation_results/{}.csv", i))
        .expect("failed to open simulation results file");
    let mut wtr = Writer::from_writer(out);

    let mut avg_delay = 0.0;
    let mut avg_k = 0.0;
    let mut i = 0;
    for record in rx_record {
        avg_delay += record.delay_until_processed;
        avg_k += record.k as f64;
        i += 1;
        wtr.serialize(record).expect("failed serialize record");
    }
    wtr.flush().expect("failed to flush writer");
    avg_delay /= i as f64;
    avg_k /= i as f64;
    println!("\tavg delay: {}ms avg k: {}", avg_delay, avg_k);
}

pub fn run_network<
    A: 'static + Send + Clone + Serializable,
    B: 'static + Send + Serializable,
    CD: 'static + Send + CongestionDetector,
    KP: 'static + Send + KPolicy,
>(
    mut network_module: NetworkModule<A, B, CD, KP>,
    op: OP,
    running: Arc<AtomicBool>,
    tx_record: Sender<Record>,
    sample_packet: A,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut i = 0;
        while running.load(Ordering::SeqCst) {
            if let Some((ts, _)) = network_module.try_recv() {
                let now = now();
                tx_record
                    .send(Record {
                        op,
                        delay_until_processed: (now - ts) as f64 / 1000.0,
                        k: network_module.k(),
                    })
                    .expect("failed to send record from slave");
            }
            if i % 4 == 0 {
                network_module.send(sample_packet.clone());
            }
            i += 1;
            thread::sleep(std::time::Duration::from_micros(250));
        }
    })
}

fn run_simulation<
    CDM: 'static + Send + CongestionDetector,
    CDS: 'static + Send + CongestionDetector,
    KPM: 'static + Send + KPolicy,
    KPS: 'static + Send + KPolicy,
>(
    i: usize,
    simulation_time: Duration,
    congestion_detector_mater: CDM,
    congestion_detector_slave: CDS,
    k_policy_master: KPM,
    k_policy_slave: KPS,
    w: f64,
    cooloff: usize,
) {
    let master = NetworkModule::<PayloadM2S, PayloadS2M, CDM, KPM>::new(
        "127.0.0.1:13380",
        "127.0.0.1:13370",
        congestion_detector_mater,
        k_policy_master,
        w,
        10,
        PayloadType::Master,
        2000.0,
    );
    let slave = NetworkModule::<PayloadS2M, PayloadM2S, CDS, KPS>::new(
        "127.0.0.1:13370",
        "127.0.0.1:13380",
        congestion_detector_slave,
        k_policy_slave,
        w,
        10,
        PayloadType::Slave,
        2000.0,
    );

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

    for (i, rate_kbs) in vec![276, 304, 360, 528, 600].into_iter().enumerate() {
        let w = 0.10;
        let cooloff = 30;

        println!("cooloff: {}, rate: {}, w: {}", cooloff, rate_kbs, w);

        /*
        let congestion_detector_mater = congestion_detection::ZigZag::new();
        let congestion_detector_slave = congestion_detection::ZigZag::new();
        */

        /*
        let congestion_detector_mater = congestion_detection::Trend::new(0.10, 0.30);
        let congestion_detector_slave = congestion_detection::Trend::new(0.10, 0.30);
        */

        let congestion_detector_mater = congestion_detection::Window::new(5);
        let congestion_detector_slave = congestion_detection::Window::new(5);

        let k_policy_master = KPolicySDMI {};
        let k_policy_slave = KPolicySDMI {};

        /*
        let k_policy_master = KPolicySDSI {};
        let k_policy_slave = KPolicySDSI {};
        */

        /*
        let k_policy_master = KPolicySDMIExponentialBackoff::new(200);
        let k_policy_slave = KPolicySDMIExponentialBackoff::new(200);
        */

        setup_network_emulator(rate_kbs, 10);
        std::thread::sleep(Duration::from_secs(3));

        run_simulation(
            i,
            simulation_time,
            congestion_detector_mater,
            congestion_detector_slave,
            k_policy_master,
            k_policy_slave,
            w,
            cooloff,
        );
    }

    Ok(())
}
