use crate::common::now;
use crate::congestion_detection::CongestionDetector;
use crate::hoip::{DelayIndicator, Header, Message, PayloadType, SamplingScheme, Serializable};
use crate::k_policy::{KPolicy, K_MAX};
use crate::network_analyzer::NetworkAnalyzer;
use crate::rate_limiter::RateLimiter;
use std::io;
use std::net::UdpSocket;

pub struct NetworkModule<S, R, CD, KP> {
    sock: UdpSocket,
    payloads: Vec<S>,
    msgs: Vec<R>,
    msgs_offset: u64,
    rott: u32,
    previous_timestamp: u64,
    network_anaylzer: NetworkAnalyzer<CD>,
    k_policy: KP,
    k: i8,
    op: PayloadType,
    rate_limiter: RateLimiter,
}

impl<S: Serializable, R: Serializable, CD: CongestionDetector, KP: KPolicy>
    NetworkModule<S, R, CD, KP>
{
    pub fn new(
        dest_addr: &str,
        src_addr: &str,
        congestion_detector: CD,
        k_policy: KP,
        w: f64,
        cooloff: usize,
        op: PayloadType,
        rate: f64,
    ) -> Self {
        let sock = UdpSocket::bind(src_addr).unwrap();
        sock.connect(dest_addr).unwrap();
        sock.set_nonblocking(true).unwrap();

        let rate_limiter = RateLimiter::new(rate);

        Self {
            sock,
            payloads: Vec::with_capacity(K_MAX as _),
            rott: 0,
            msgs: Vec::new(),
            previous_timestamp: 0,
            msgs_offset: 0,
            network_anaylzer: NetworkAnalyzer::new(congestion_detector, w),
            k_policy,
            k: K_MAX,
            op,
            rate_limiter,
        }
    }

    pub fn send(&mut self, payload: S) {
        let state = self.network_anaylzer.state();
        if let Some(new_k) = self.k_policy.select_k(state, self.k) {
            self.k = new_k;
        }

        self.payloads.push(payload);
        if self.payloads.len() < self.k as _ {
            return;
        }

        let too_many = self.payloads.len() - self.k as usize;
        if too_many > 0 {
            self.payloads.drain(0..too_many);
        }

        if self.rate_limiter.limited() {
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
            self.msgs_offset -= 1;
            let ts = self.previous_timestamp + self.msgs_offset * 1000;
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
                    panic!("{:}", e);
                }
                v => num_bytes = v.unwrap(),
            };
        }
        if num_bytes == 0 {
            return self.try_pop_msg();
        }
        let msg = Message::from_bytes(&buf[0..num_bytes]);
        self.rott = (now() - msg.timestamp()) as _;
        if self.previous_timestamp < msg.timestamp() {
            self.msgs = msg
                .payload
                .chunks(R::len())
                .map(|bs| R::from_bytes(bs))
                .collect();
            self.msgs_offset = self.msgs.len() as u64;

            self.network_anaylzer
                .update_state(msg.rott() + 1000 * (self.msgs_offset - 1) as u32);

            self.previous_timestamp = msg.timestamp();
        }
        self.try_pop_msg()
    }

    pub fn k(&self) -> i8 {
        self.k
    }

    pub fn rate(&self) -> f64 {
        self.rate_limiter.rate()
    }

    pub fn set_rate(&mut self, rate: f64) {
        self.rate_limiter.set_rate(rate);
    }
}
