use bitvec::prelude::*;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read, Write};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PayloadType {
    Master,
    Slave,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SamplingScheme {
    Lossless,
    Weber,
    LevelCrossing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DelayIndicator {
    InHeader,
    InPayload,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub payload_type: PayloadType,
    pub sampling_scheme: SamplingScheme,
    pub num_samples: u8,
    pub delay_indicator: DelayIndicator,
    pub threshold: u16,
    pub notification_delay: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub header: Header,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayloadS2M {
    force: [f32; 3],
}

impl PayloadS2M {
    pub fn new(force: [f32; 3]) -> Self {
        Self { force }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayloadM2S {
    pos: [f32; 3],
    vel: [f32; 3],
}

impl PayloadM2S {
    pub fn new(pos: [f32; 3], vel: [f32; 3]) -> Self {
        Self { pos, vel }
    }
}

pub trait Serializable {
    fn len() -> usize;

    fn from_bytes(bs: &[u8]) -> Self;

    fn to_bytes(self) -> Vec<u8>;
}

impl Serializable for PayloadM2S {
    fn to_bytes(self) -> Vec<u8> {
        let mut wtr = Vec::with_capacity(3 * 8);
        for f in self.pos.iter() {
            wtr.write_f32::<BigEndian>(*f).unwrap();
        }
        for f in self.vel.iter() {
            wtr.write_f32::<BigEndian>(*f).unwrap();
        }
        wtr
    }

    fn from_bytes(bs: &[u8]) -> Self {
        let mut rdr = Cursor::new(bs);
        let mut pos = [0.0f32; 3];
        let mut vel = [0.0f32; 3];
        rdr.read_f32_into::<BigEndian>(&mut pos).unwrap();
        rdr.read_f32_into::<BigEndian>(&mut vel).unwrap();
        Self { pos, vel }
    }

    fn len() -> usize {
        3 * 4 + 3 * 4
    }
}

impl Serializable for PayloadS2M {
    fn to_bytes(self) -> Vec<u8> {
        let mut wtr = Vec::with_capacity(3 * 8);
        for f in self.force.iter() {
            wtr.write_f32::<BigEndian>(*f).unwrap();
        }
        wtr
    }

    fn from_bytes(bs: &[u8]) -> Self {
        let mut rdr = Cursor::new(bs);
        let mut force = [0.0; 3];
        rdr.read_f32_into::<BigEndian>(&mut force).unwrap();
        Self { force }
    }

    fn len() -> usize {
        3 * 4
    }
}

impl Message {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut wtr = Vec::with_capacity(14 + self.payload.len());

        let mut byte = 0;
        let bits = byte.bits_mut::<bitvec::cursor::BigEndian>();
        match self.header.payload_type {
            PayloadType::Master => *bits.at(0) = false,
            PayloadType::Slave => *bits.at(0) = true,
        };
        match self.header.sampling_scheme {
            SamplingScheme::Lossless => {}
            SamplingScheme::Weber => *bits.at(3) = true,
            SamplingScheme::LevelCrossing => *bits.at(4) = true,
        };
        match self.header.num_samples {
            1 => {}
            2 => *bits.at(5) = true,
            3 => *bits.at(6) = true,
            4 => {
                *bits.at(5) = true;
                *bits.at(6) = true;
            }
            _ => unreachable!(),
        }
        match self.header.delay_indicator {
            DelayIndicator::InHeader => {}
            DelayIndicator::InPayload => *bits.at(7) = true,
        };
        byte |= (self.header.num_samples ^ 0b00000011) >> 5;

        wtr.write_u8(byte).unwrap();
        wtr.write_u16::<BigEndian>(self.header.threshold).unwrap();
        let notification_delay = std::cmp::min(0xFFFFFF, self.header.notification_delay);
        wtr.write_u24::<BigEndian>(notification_delay).unwrap();
        wtr.write_u64::<BigEndian>(self.header.timestamp).unwrap();

        wtr.write_all(&self.payload).unwrap();

        wtr
    }

    pub fn from_bytes(bs: &[u8]) -> Self {
        let mut rdr = Cursor::new(bs);
        let byte = rdr.read_u8().unwrap();
        let bits = byte.bits::<bitvec::cursor::BigEndian>();
        let payload_type = match bits[0] {
            false => PayloadType::Master,
            true => PayloadType::Slave,
        };
        let sampling_scheme = match (bits[4], bits[3]) {
            (false, false) => SamplingScheme::Lossless,
            (false, true) => SamplingScheme::Weber,
            (true, false) => SamplingScheme::LevelCrossing,
            _ => unimplemented!(),
        };
        let num_samples = match (bits[6], bits[5]) {
            (false, false) => 1,
            (false, true) => 2,
            (true, false) => 3,
            (true, true) => 4,
        };
        let delay_indicator = match bits[7] {
            false => DelayIndicator::InHeader,
            true => DelayIndicator::InPayload,
        };

        let threshold = rdr.read_u16::<BigEndian>().unwrap();
        let notification_delay = rdr.read_u24::<BigEndian>().unwrap();
        let timestamp = rdr.read_u64::<BigEndian>().unwrap();

        let mut payload = Vec::with_capacity(bs.len() - 14);
        rdr.read_to_end(&mut payload).unwrap();

        Self {
            header: Header {
                payload_type,
                sampling_scheme,
                num_samples,
                delay_indicator,
                threshold,
                notification_delay,
                timestamp,
            },
            payload,
        }
    }

    pub fn notification_delay(&self) -> u32 {
        self.header.notification_delay
    }

    pub fn timestamp(&self) -> u64 {
        self.header.timestamp
    }

    pub fn num_samples(&self) -> u8 {
        self.header.num_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        for payload_type in vec![PayloadType::Master, PayloadType::Slave].into_iter() {
            for sampling_scheme in vec![
                SamplingScheme::Weber,
                SamplingScheme::LevelCrossing,
                SamplingScheme::Lossless,
            ]
            .into_iter()
            {
                for num_samples in 1..5 {
                    for delay_indicator in
                        vec![DelayIndicator::InHeader, DelayIndicator::InPayload].into_iter()
                    {
                        let msg = Message {
                            header: Header {
                                payload_type,
                                sampling_scheme,
                                num_samples,
                                delay_indicator,
                                threshold: 10,
                                notification_delay: 1,
                                timestamp: std::u64::MAX,
                            },
                            payload: vec![1, 2, 3],
                        };
                        assert_eq!(msg.clone(), Message::from_bytes(&msg.to_bytes()));
                    }
                }
            }
        }
    }
}
