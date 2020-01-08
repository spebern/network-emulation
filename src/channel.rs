use crate::path_loss::GilbertElliot;
use async_std::task;
use std::sync::{
    atomic::{AtomicBool, Ordering::SeqCst},
    Arc,
};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Packet {
    dummy_len: Option<usize>,
    payload: Vec<u8>,
}

impl Packet {
    fn len(&self) -> usize {
        let payload_len = if let Some(dummy_len) = self.dummy_len {
            dummy_len
        } else {
            self.payload.len()
        };
        8 + 20 + payload_len
    }

    fn set_dummy_len(&mut self, dummy_len: usize) {
        self.dummy_len = Some(dummy_len);
    }

    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            dummy_len: None,
            payload,
        }
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

impl Default for Packet {
    fn default() -> Self {
        Packet {
            dummy_len: None,
            payload: Vec::new(),
        }
    }
}

pub struct Contender {
    running: Arc<AtomicBool>,
}

impl Drop for Contender {
    fn drop(&mut self) {
        self.running.store(false, SeqCst);
    }
}

pub struct Rx(mpsc::UnboundedReceiver<Packet>);

impl Rx {
    pub async fn recv(&mut self) -> Option<Packet> {
        self.0.recv().await
    }

    pub fn try_recv(&mut self) -> Option<Packet> {
        self.0.try_recv().ok()
    }
}

#[derive(Clone)]
pub struct Tx {
    path_loss_model: GilbertElliot,
    transmission_delay: Duration,
    tx_queue: mpsc::UnboundedSender<Packet>,
}

impl Tx {
    fn new(
        tx_packet: mpsc::UnboundedSender<Packet>,
        path_loss_model: GilbertElliot,
        transmission_delay: Duration,
        capacity: f64,
    ) -> Self {
        let (tx_queue, rx_queue) = mpsc::unbounded_channel();
        task::spawn(Self::process_queued_packets(rx_queue, tx_packet, capacity));
        Self {
            path_loss_model,
            transmission_delay,
            tx_queue,
        }
    }

    fn add_tcp_contender(&self, capacity: f64) -> Contender {
        let tx_queue = self.tx_queue.clone();
        let running = Arc::new(AtomicBool::new(true));
        {
            let running = running.clone();
            task::spawn(async move {
                while running.load(SeqCst) {
                    let mut packet = Packet::default();
                    packet.set_dummy_len(500);
                    let delay =
                        Duration::from_micros(((packet.len() * 1_000_000) as f64 / capacity) as _);
                    let _ = tx_queue.send(packet);
                    task::sleep(delay).await;
                }
            });
        }
        Contender { running }
    }

    async fn process_queued_packets(
        mut rx_packet: mpsc::UnboundedReceiver<Packet>,
        tx_msgs: mpsc::UnboundedSender<Packet>,
        capacity: f64,
    ) {
        while let Some(packet) = rx_packet.recv().await {
            let delay =
                Duration::from_micros((((packet.len() * 8 * 1_000_000) as f64) / capacity) as _);
            //task::sleep(delay).await;
            task::sleep(Duration::from_micros(500)).await;
            let _ = tx_msgs.send(packet);
        }
    }

    pub async fn send(&self, packet: Packet) {
        if self.path_loss_model.transmit().await {
            let tx_queue = self.tx_queue.clone();
            let transmission_delay = self.transmission_delay;
            task::spawn(async move {
                task::sleep(transmission_delay).await;
                let _ = tx_queue.send(packet).ok();
            });
        }
    }
}

pub fn channel(
    path_loss_model: GilbertElliot,
    transmission_delay: Duration,
    capacity: f64,
) -> (Tx, Rx) {
    let (tx_packet, rx_packet) = mpsc::unbounded_channel();
    (
        Tx::new(tx_packet, path_loss_model, transmission_delay, capacity),
        Rx(rx_packet),
    )
}
