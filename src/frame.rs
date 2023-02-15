use bitvec::prelude::{BitVec, Lsb0};
use crc::{Crc, CRC_16_ISO_IEC_14443_3_A};
use std::cmp::Ordering;

use crate::error::FrameError;

pub const EDC_CALC: Crc<u16> = Crc::<u16>::new(&CRC_16_ISO_IEC_14443_3_A);

#[derive(Debug, Eq, PartialEq)]
pub enum Frame {
    Short(u8),
    StandardNoCrc(Vec<u8>),
    StandardCrc(Vec<u8>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct CompleteCollector {
    pub(crate) data: BitVec<u8, Lsb0>,
}

impl CompleteCollector {
    pub fn to_frame(&self) -> Result<Frame, FrameError> {
        let data_len = self.data.len();
        match data_len.cmp(&8) {
            Ordering::Greater => {
                if data_len % 9 == 0 {
                    let total_bytes = data_len / 9;
                    let mut out: Vec<u8> = Vec::with_capacity(total_bytes);
                    for byte_number in 0..total_bytes {
                        let mut byte: u8 = 0;
                        let mut expected_parity_bit = true;
                        for (i, bit) in self.data[byte_number * 9..byte_number * 9 + 8]
                            .iter()
                            .enumerate()
                        {
                            if *bit {
                                expected_parity_bit = !expected_parity_bit;
                                byte |= 1 << i;
                            }
                        }
                        if expected_parity_bit != self.data[byte_number * 9 + 8] {
                            return Err(FrameError::ParityBit);
                        } else {
                            out.push(byte)
                        }
                    }
                    if total_bytes < 3 {
                        Ok(Frame::StandardNoCrc(out))
                    } else {
                        let crc = u16::from_le_bytes(
                            out[total_bytes - 2..]
                                .try_into()
                                .expect("static length, always fits"),
                        );
                        let out = out[..total_bytes - 2].to_vec();
                        if EDC_CALC.checksum(&out) == crc {
                            Ok(Frame::StandardCrc(out))
                        } else {
                            Err(FrameError::CrcMismatch)
                        }
                    }
                } else {
                    Err(FrameError::LengthNot9Multiple)
                }
            }
            Ordering::Equal => Err(FrameError::Length8Bit),
            Ordering::Less => {
                let byte: u8 = self.data.to_owned().into_vec()[0];
                Ok(Frame::Short(byte))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::prelude::bitvec;

    #[test]
    fn wrap_collector_1() {
        let complete_collector = CompleteCollector {
            data: bitvec![u8, Lsb0; 0, 1, 1, 0, 0, 1, 0],
        };
        let frame = complete_collector.to_frame().unwrap();
        assert_eq!(frame, Frame::Short(0x26));
    }

    #[test]
    fn wrap_collector_2() {
        let complete_collector = CompleteCollector {
            data: bitvec![u8, Lsb0; 0, 1, 0, 0, 1, 0, 1],
        };
        let frame = complete_collector.to_frame().unwrap();
        assert_eq!(frame, Frame::Short(0x52));
    }

    #[test]
    fn wrap_collector_3() {
        let complete_collector = CompleteCollector {
            data: bitvec![u8, Lsb0; 0, 0, 0, 0, 1, 0, 1, 0, 1],
        };
        let frame = complete_collector.to_frame().unwrap();
        assert_eq!(frame, Frame::StandardNoCrc(vec![0x50]));
    }

    #[test]
    fn wrap_collector_4() {
        let complete_collector = CompleteCollector {
            data: bitvec![u8, Lsb0; 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0],
        };
        let frame = complete_collector.to_frame().unwrap();
        assert_eq!(frame, Frame::StandardCrc(vec![0x50, 0x00]));
    }
}