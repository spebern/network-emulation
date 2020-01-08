use crate::{
    channel::{Packet, Rx, Tx},
    common::timestamp,
    config::ChannelConfig,
    hoip::{DelayIndicator, Header, Message, PayloadType, SamplingScheme, Serializable},
};
use async_std::{
    sync::{Arc, Mutex},
    task,
};
use std::sync::atomic::{AtomicI8, AtomicU32, Ordering::SeqCst};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const K_MAX: i8 = 4;
const K_MIN: i8 = 1;

pub struct Network<S, R> {
    // The outgoing packets.
    tx_packet: Tx,
    // The compression factor.
    k: Arc<AtomicI8>,
    // The buffer used for storing merged packets.
    payloads: Vec<S>,
    // The latest notification delay.
    notification_delay: Arc<AtomicU32>,
    // The incoming messages.
    rx_msgs: UnboundedReceiver<(u64, R)>,
    // The config of the channel between sender and receiver.
    channel_config: ChannelConfig,
    // Whether to adjust k according to the congestion or not.
    adjust_k: bool,
    // The type of the payload.
    op: PayloadType,
}

struct NetworkAnalyzer {
    // The weight used for the exponential decaying average.
    w: f64,
    // Number of consecutive increasing delays.
    increasing_delays_in_a_row: isize,
    // The number of triggers in a row required to adapt `k`.
    N: isize,
    // The estimated average delay.
    avg_delay: f64,
    // The compression ratio.
    k: Arc<AtomicI8>,
    // A counter that starts after each state reset.
    counter: isize,
    // The previous `N` notification delays.
    previous_notification_delays: Vec<u64>,
}

fn dur_max() -> Duration {
    Duration::new(std::u64::MAX, 1_000_000_000 - 1)
}

impl NetworkAnalyzer {
    fn new(w: f64, N: isize, k: Arc<AtomicI8>) -> Self {
        Self {
            w,
            increasing_delays_in_a_row: 0,
            N,
            avg_delay: dur_max().as_micros() as _,
            k,
            counter: 0,
            previous_notification_delays: (0..N).map(|_| std::u64::MAX).collect(),
        }
    }

    fn add_to_k(&self, i: i8) {
        loop {
            let k = self.k.load(SeqCst);
            let new_k = if i < 0 {
                std::cmp::max(K_MIN, k + i)
            } else {
                std::cmp::min(K_MAX, k + i)
            };
            if k > 1 {
                if k == self.k.compare_and_swap(k, new_k, SeqCst) {
                    break;
                }
            } else {
                break;
            }
        }
    }

    async fn update_state(&mut self, notification_delay: u64) {
        // calculate new average delay
        let avg_delay = (1.0 - self.w) * self.avg_delay + self.w * notification_delay as f64;
        if avg_delay > self.avg_delay {
            self.increasing_delays_in_a_row += 1;
        } else {
            self.increasing_delays_in_a_row = 0;
        }
        self.avg_delay = avg_delay;

        self.previous_notification_delays[(self.counter % self.N) as usize] = notification_delay;

        // count up the rounds after last state change
        self.counter += 1;

        // early return if we have not collected enough samples to decide on state change
        if self.counter < self.N {
            return;
        }

        // check if we exceeded the number of increasing delays in a row
        if self.increasing_delays_in_a_row > self.N {
            // maximum throttle
            self.k.store(K_MAX, SeqCst);
            self.reset();
            return;
        }

        let sum: f64 = self
            .previous_notification_delays
            .iter()
            .fold(0.0, |acc, x| acc + *x as f64);
        let avg = sum / self.previous_notification_delays.len() as f64;
        let delay = notification_delay;
        let decreasing_notification_delays = 0;
        for i in self.counter..self.counter + self.N {
            let i = (i % self.N) as usize;
            let delay = self.previous_notification_delays[i] as f64;
            if delay < 0.9 * avg || delay > 1.1 * avg {
                return;
            }
        }

        // if we don't have an increasing or decreasing tend try to increase k
        //if decreasing_notification_delays < self.N {
        self.add_to_k(-1);
        self.reset();
        //}
    }

    fn reset(&mut self) {
        self.counter = 0;
        for dur in self.previous_notification_delays.iter_mut() {
            *dur = std::u64::MAX;
        }
    }
}

impl<S: 'static + Send + Serializable, R: 'static + Send + Serializable> Network<S, R> {
    pub fn new(
        tx_packet: Tx,
        rx_packet: Rx,
        w: f64,
        N: isize,
        channel_config: ChannelConfig,
        adjust_k: bool,
        op: PayloadType,
    ) -> Self {
        assert!(w <= 1.0 && w >= 0.0);

        let k = Arc::new(AtomicI8::new(K_MAX));
        let notification_delay = Arc::new(AtomicU32::new(std::u32::MAX));
        let network_analyzer = NetworkAnalyzer::new(w, N, k.clone());
        let (tx_msgs, rx_msgs) = unbounded_channel();
        task::spawn(Self::handle_responses(
            rx_packet,
            network_analyzer,
            tx_msgs,
            notification_delay.clone(),
        ));

        Self {
            k,
            tx_packet,
            payloads: Vec::with_capacity(K_MAX as _),
            notification_delay,
            rx_msgs,
            channel_config,
            adjust_k,
            op,
        }
    }

    async fn handle_responses(
        mut rx_packets: Rx,
        mut network_analyzer: NetworkAnalyzer,
        tx_msgs: UnboundedSender<(u64, R)>,
        notification_delay: Arc<AtomicU32>,
    ) {
        let mut prev_ts = 0;
        while let Some(packet) = rx_packets.recv().await {
            let msg = Message::from_bytes(packet.payload());
            notification_delay.store(msg.notification_delay(), SeqCst);
            let now = timestamp();
            network_analyzer.update_state(now - msg.timestamp()).await;

            // only consider newer sequence numbers
            if msg.timestamp() > prev_ts {
                prev_ts = msg.timestamp();
                for (i, bs) in msg.payload.chunks(R::len()).enumerate() {
                    let _ = tx_msgs.send((msg.timestamp() + i as u64 * 800, R::from_bytes(bs)));
                    task::sleep(Duration::from_micros(800)).await;
                }
            }
        }
    }

    pub fn send(&mut self, payload: S) {
        let k = if self.adjust_k {
            //self.k.load(SeqCst)
            1
        } else {
            1
        };

        self.payloads.push(payload);
        if self.payloads.len() < k as _ {
            return;
        }

        let payloads = std::mem::replace(&mut self.payloads, Vec::with_capacity(K_MAX as _));
        let tx_packets = self.tx_packet.clone();
        let notification_delay = self.notification_delay.clone();
        let op = self.op;
        task::spawn(async move {
            let notification_delay = notification_delay.load(SeqCst);
            let num_samples = payloads.len() as u8;
            let payload = payloads
                .into_iter()
                .map(|m| m.to_bytes())
                .collect::<Vec<_>>()
                .concat();
            let msg = Message {
                header: Header {
                    payload_type: op,
                    sampling_scheme: SamplingScheme::Lossless,
                    num_samples,
                    delay_indicator: DelayIndicator::InHeader,
                    threshold: 10,
                    notification_delay,
                    timestamp: timestamp(),
                },
                payload,
            }
            .to_bytes();
            let packet = Packet::new(msg);
            tx_packets.send(packet).await;
        });
    }

    pub fn k(&self) -> i8 {
        self.k.load(SeqCst)
    }

    pub fn try_recv(&mut self) -> Option<(u64, R)> {
        let mut newest = None;
        while let Some(v) = self.rx_msgs.try_recv().ok() {
            newest = Some(v);
        }
        newest
    }

    pub fn channel_config(&self) -> &ChannelConfig {
        &self.channel_config
    }

    pub fn adjust_k(&self) -> bool {
        self.adjust_k
    }
}
