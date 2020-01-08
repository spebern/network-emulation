use crate::common::timestamp;
use crate::hoip::{DelayIndicator, Header, Message, PayloadType, SamplingScheme, Serializable};
use std::cmp::{max, min};
use std::io;
use std::net::UdpSocket;

const MASTER_ADDR: &'static str = "127.0.0.1:13370";
const SLAVE_ADDR: &'static str = "127.0.0.1:13380";

const K_MAX: i8 = 4;
const K_MIN: i8 = 1;

struct NetworkAnalyzer {
    // The weight used for the exponential decaying average.
    w: f64,
    // Number of consecutive increasing delays.
    increasing_rotts_in_a_row: isize,
    // The number of triggers in a row required to adapt `k`.
    n: isize,
    // The estimated average delay.
    avg_rott: f64,
    // The estimated standard deviation of the delay.
    std_rott: f64,
    // The compression ratio.
    k: i8,
    // A counter that starts after each state reset.
    counter: isize,
    // The previous `N` notification delays.
    previous_rotts: Vec<u32>,
    // The previous rott.
    prev_rott: u32,
    // The method based upon which `k` will be selected.
    k_select_method: KSelectMethod,
    // The current trend index.
    s_f: f64,
    // The threshold of the trend index.
    s_threshold: f64,
    // Exponential decaying factor that weights the impact of the current `rott` on `s_f`.
    gamma: f64,
}

#[derive(Debug, Clone, Copy)]
enum KSelectMethod {
    ZigZag,
    Biaz, // would only work well if no Weber or some adaptions for Weber
    Trend,
    Window,
}

impl NetworkAnalyzer {
    fn new(w: f64, n: isize, k_select_method: KSelectMethod) -> Self {
        Self {
            w,
            increasing_rotts_in_a_row: 0,
            n,
            avg_rott: 0.0,
            std_rott: 0.0,
            prev_rott: 0,
            k: K_MAX,
            counter: 0,
            previous_rotts: (0..n).map(|_| std::u32::MAX).collect(),
            k_select_method,
            s_f: 0.0,
            s_threshold: 0.4,
            gamma: 0.4,
        }
    }

    fn calc_avg_and_std_rott(&self, rott: u32) -> (f64, f64) {
        let avg_rott = (1.0 - self.w) * self.avg_rott + self.w * rott as f64;
        let std_rott = (1.0 - 2.0 * self.w) + 2.0 * self.w * (rott as f64 - avg_rott).abs();
        (avg_rott, std_rott)
    }

    fn select_k_zig_zag(&mut self, rott: u32, avg_rott: f64, std_rott: f64) {
        self.k = if rott as f64 > avg_rott + std_rott {
            max(K_MAX, self.k + 1)
        } else {
            min(K_MIN, self.k - 1)
        };
    }

    fn select_k_trend(&mut self, rott: u32) {
        self.s_f = (1.0 - self.gamma) * self.s_f;
        if rott > self.prev_rott {
            self.s_f += self.gamma;
        }
        self.k = if self.s_threshold < self.s_f {
            max(K_MAX, self.k + 1)
        } else {
            min(K_MIN, self.k - 1)
        };
    }

    fn select_k_window(&mut self, rott: u32, avg_rott: f64) {
        if rott as f64 > avg_rott {
            self.increasing_rotts_in_a_row += 1;
        } else {
            self.increasing_rotts_in_a_row = 0;
        }

        self.previous_rotts[(self.counter % self.n) as usize] = rott;

        // count up the rounds after last state change
        self.counter += 1;

        // check if we exceeded the number of increasing delays in a row
        if self.increasing_rotts_in_a_row > self.n {
            self.reset();
            // maximum throttle
            self.k = K_MAX;
            return;
        }

        let sum: f64 = self
            .previous_rotts
            .iter()
            .fold(0.0, |acc, x| acc + *x as f64);
        let avg = sum / self.previous_rotts.len() as f64;
        let mut rott = rott;
        let mut increasing_rotts = 0;
        for i in self.counter + 1..self.counter + self.n + 1 {
            if rott < (0.95 * avg) as _ || rott > (1.05 * avg) as _ {
                return;
            }
            let i = (i % self.n) as usize;
            let next_rott = self.previous_rotts[i];
            if rott < next_rott {
                increasing_rotts += 1;
            }
            rott = next_rott;
        }

        // if we don't have an increasing or decreasing tend try to increase k
        if increasing_rotts < self.n {
            self.k = std::cmp::max(K_MIN, self.k - 1);
        }
    }

    fn update_state(&mut self, rott: u32) {
        let (avg_rott, std_rott) = self.calc_avg_and_std_rott(rott);

        match self.k_select_method {
            KSelectMethod::ZigZag => self.select_k_zig_zag(rott, avg_rott, std_rott),
            KSelectMethod::Biaz => unimplemented!(),
            KSelectMethod::Trend => self.select_k_trend(rott),
            KSelectMethod::Window => self.select_k_window(rott, avg_rott),
        };
        self.avg_rott = avg_rott;
        self.std_rott = std_rott;
        self.prev_rott = rott;
    }

    fn reset(&mut self) {
        self.counter = 0;
        for dur in self.previous_rotts.iter_mut() {
            *dur = std::u32::MAX;
        }
    }
}

pub struct NetworkModule<S, R> {
    sock: UdpSocket,
    adjust_k: bool,
    payloads: Vec<S>,
    msgs: Vec<R>,
    msgs_offset: u64,
    rott: u32,
    op: PayloadType,
    previous_timestamp: u64,
    network_anaylzer: NetworkAnalyzer,
}

impl<S: Serializable, R: Serializable> NetworkModule<S, R> {
    pub fn new(op: PayloadType, adjust_k: bool) -> Self {
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
            network_anaylzer: NetworkAnalyzer::new(0.10, 4, KSelectMethod::Window),
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
                notification_delay: rott,
                timestamp: timestamp(),
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
        self.network_anaylzer.update_state(msg.notification_delay());
        self.rott = (timestamp() - msg.timestamp()) as _;
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
