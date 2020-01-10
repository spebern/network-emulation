use crate::hoip::{DelayIndicator, Header, Message, PayloadType, SamplingScheme, Serializable};
use crate::now;
use std::io;
use std::net::UdpSocket;

const MASTER_ADDR: &'static str = "127.0.0.1:13370";
const SLAVE_ADDR: &'static str = "127.0.0.1:13380";

pub const K_MAX: i8 = 4;
pub const K_MIN: i8 = 1;

struct NetworkAnalyzer<K> {
    // The estimated average delay.
    avg_rott: f64,
    // The estimated standard deviation of the delay.
    std_rott: f64,
    // The compression ratio.
    k: i8,
    // The previous rott.
    prev_rott: u32,
    // The selector for choosing the most suitable `k`.
    k_selector: K,
    // The weight of the exponential decaying average used for
    // calculating the average delay and its variance.
    w: f64,
}

pub trait KSelector {
    fn select_k(
        &mut self,
        current_k: i8,
        rott: u32,
        avg_rott: f64,
        std_rott: f64,
        prev_rott: u32,
    ) -> i8;
}

impl<K: KSelector> NetworkAnalyzer<K> {
    fn new(k_selector: K, w: f64) -> Self {
        Self {
            avg_rott: 0.0,
            std_rott: 0.0,
            prev_rott: 0,
            k: K_MAX,
            k_selector,
            w,
        }
    }

    fn calc_avg_and_std_rott(&self, rott: u32) -> (f64, f64) {
        let avg_rott = (1.0 - self.w) * self.avg_rott + self.w * rott as f64;
        let std_rott = (1.0 - 2.0 * self.w) + 2.0 * self.w * (rott as f64 - avg_rott).abs();
        (avg_rott, std_rott)
    }

    fn update_state(&mut self, rott: u32) {
        let (avg_rott, std_rott) = self.calc_avg_and_std_rott(rott);

        self.k = self
            .k_selector
            .select_k(self.k, rott, avg_rott, std_rott, self.prev_rott);

        self.avg_rott = avg_rott;
        self.std_rott = std_rott;
        self.prev_rott = rott;
    }
}

pub struct NetworkModule<S, R, K> {
    sock: UdpSocket,
    adjust_k: bool,
    payloads: Vec<S>,
    msgs: Vec<R>,
    msgs_offset: u64,
    rott: u32,
    op: PayloadType,
    previous_timestamp: u64,
    network_anaylzer: NetworkAnalyzer<K>,
}

impl<S: Serializable, R: Serializable, K: KSelector> NetworkModule<S, R, K> {
    pub fn new(op: PayloadType, adjust_k: bool, k_selector: K, w: f64) -> Self {
        let sock = match op {
            PayloadType::Master => {
                let sock = UdpSocket::bind(MASTER_ADDR).unwrap();
                sock.connect(SLAVE_ADDR).unwrap();
                sock
            }
            PayloadType::Slave => {
                let sock = UdpSocket::bind(SLAVE_ADDR).unwrap();
                sock.connect(MASTER_ADDR).unwrap();
                sock
            }
        };
        sock.set_nonblocking(true).unwrap();
        Self {
            sock,
            adjust_k,
            payloads: Vec::with_capacity(K_MAX as _),
            rott: 0,
            op,
            msgs: Vec::new(),
            previous_timestamp: 0,
            msgs_offset: 0,
            network_anaylzer: NetworkAnalyzer::new(k_selector, w),
        }
    }

    pub fn send(&mut self, payload: S) {
        let k = if self.adjust_k {
            self.network_anaylzer.k
        } else {
            1
        };

        self.payloads.push(payload);
        if self.payloads.len() < k as _ {
            return;
        }

        let payloads = std::mem::replace(&mut self.payloads, Vec::with_capacity(K_MAX as _));
        let rott = self.rott;
        let num_samples = payloads.len() as u8;
        let payload = payloads
            .into_iter()
            .map(|m| m.to_bytes())
            .collect::<Vec<_>>()
            .concat();
        let msg = Message {
            header: Header {
                payload_type: self.op,
                sampling_scheme: SamplingScheme::Lossless,
                num_samples,
                delay_indicator: DelayIndicator::InHeader,
                threshold: 10,
                rott,
                timestamp: now(),
            },
            payload,
        }
        .to_bytes();
        self.sock.send(&msg).unwrap();
    }

    fn try_pop_msg(&mut self) -> Option<(u64, R)> {
        self.msgs.pop().map(|msgs| {
            let ts = self.previous_timestamp + self.msgs_offset * 1000;
            self.msgs_offset += 1;
            (ts, msgs)
        })
    }

    pub fn try_recv(&mut self) -> Option<(u64, R)> {
        let mut buf = [0; 300];
        let mut num_bytes = 0;
        loop {
            match self.sock.recv(&mut buf) {
                Err(e) => {
                    if let io::ErrorKind::WouldBlock = e.kind() {
                        break;
                    }
                    unreachable!();
                }
                v => num_bytes = v.unwrap(),
            };
        }
        if num_bytes == 0 {
            return self.try_pop_msg();
        }
        let msg = Message::from_bytes(&buf[0..num_bytes]);
        self.network_anaylzer.update_state(msg.rott());
        self.rott = (now() - msg.timestamp()) as _;
        if self.previous_timestamp < msg.timestamp() {
            self.msgs_offset = 0;
            self.msgs = msg
                .payload
                .chunks(R::len())
                .map(|bs| R::from_bytes(bs))
                .collect();
            self.previous_timestamp = msg.timestamp();
        }
        self.try_pop_msg()
    }

    pub fn adjust_k(&self) -> bool {
        self.adjust_k
    }

    pub fn k(&self) -> i8 {
        self.network_anaylzer.k
    }
}
