// --- S/PDIF (IEC 61937) Constants ---
// Syncwords in little-endian byte order.
const SYNCWORD_PA: u16 = 0xF872;
const SYNCWORD_PB: u16 = 0x4E1F;
const IEC61937_DATA_TYPE_EAC3: u8 = 0x15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iec61937Packet {
    pub data_type: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
enum ParserState {
    WaitingForSync,
    WaitingForHeader,
    WaitingForPayload {
        data_type: u8,
        payload_size: usize,
        pd_raw: u16,
    },
}

/// IEC 61937 S/PDIF parser that extracts transport packets.
pub struct SpdifParser {
    buffer: Vec<u8>,
    state: ParserState,
}

impl SpdifParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(256 * 1024),
            state: ParserState::WaitingForSync,
        }
    }

    /// Reset parser state - call when seeking/discontinuity occurs.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.state = ParserState::WaitingForSync;
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Process buffered bytes and extract the next complete IEC 61937 packet.
    pub fn get_next_packet(&mut self) -> Option<Iec61937Packet> {
        loop {
            match self.state {
                ParserState::WaitingForSync => {
                    let sync_pos = self.buffer.windows(4).position(|w| {
                        u16::from_le_bytes([w[0], w[1]]) == SYNCWORD_PA
                            && u16::from_le_bytes([w[2], w[3]]) == SYNCWORD_PB
                    });

                    match sync_pos {
                        Some(pos) => {
                            if pos > 0 {
                                self.buffer.drain(0..pos);
                            }
                            self.state = ParserState::WaitingForHeader;
                        }
                        None => {
                            let keep_len = self.buffer.len().min(3);
                            if self.buffer.len() > keep_len {
                                self.buffer.drain(0..self.buffer.len() - keep_len);
                            }
                            return None;
                        }
                    }
                }
                ParserState::WaitingForHeader => {
                    if self.buffer.len() < 8 {
                        return None;
                    }

                    let data_type = self.buffer[4] & 0x1F; // bits 0-4 of Pc
                    let pd_raw = u16::from_le_bytes([self.buffer[6], self.buffer[7]]);
                    let (payload_size, payload_unit) = payload_size_from_pd(data_type, pd_raw);
                    log::debug!(
                        "IEC 61937 header: data_type=0x{:02X} pd_raw={} payload_size={} payload_unit={}",
                        data_type,
                        pd_raw,
                        payload_size,
                        payload_unit
                    );
                    self.buffer.drain(0..8);
                    self.state = ParserState::WaitingForPayload {
                        data_type,
                        payload_size,
                        pd_raw,
                    };
                }
                ParserState::WaitingForPayload {
                    data_type,
                    payload_size,
                    pd_raw,
                } => {
                    if self.buffer.len() < payload_size {
                        return None;
                    }

                    let payload = self.buffer.drain(0..payload_size).collect::<Vec<u8>>();
                    self.state = ParserState::WaitingForSync;
                    log::debug!(
                        "IEC 61937 packet extracted: data_type=0x{:02X} pd_raw={} payload={} bytes",
                        data_type,
                        pd_raw,
                        payload.len()
                    );
                    return Some(Iec61937Packet { data_type, payload });
                }
            }
        }
    }
}

fn payload_size_from_pd(data_type: u8, pd_raw: u16) -> (usize, &'static str) {
    if data_type == IEC61937_DATA_TYPE_EAC3 {
        (usize::from(pd_raw), "bytes")
    } else {
        (usize::from(pd_raw), "bytes")
    }
}

impl Default for SpdifParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Iec61937Packet, SpdifParser};

    #[test]
    fn extracts_single_packet() {
        let mut parser = SpdifParser::new();
        let packet = [
            0x72, 0xF8, 0x1F, 0x4E, 0x16, 0x00, 0x04, 0x00, 0xAA, 0xBB, 0xCC, 0xDD,
        ];
        parser.push_bytes(&packet);
        assert_eq!(
            parser.get_next_packet(),
            Some(Iec61937Packet {
                data_type: 0x16,
                payload: vec![0xAA, 0xBB, 0xCC, 0xDD],
            })
        );
        assert_eq!(parser.get_next_packet(), None);
    }

    #[test]
    fn eac3_pd_is_payload_bytes() {
        let mut parser = SpdifParser::new();
        let packet = [
            0x72, 0xF8, 0x1F, 0x4E, 0x15, 0x00, 0x04, 0x00, 0x0B, 0x77, 0xAA, 0xBB,
        ];
        parser.push_bytes(&packet);
        assert_eq!(
            parser.get_next_packet(),
            Some(Iec61937Packet {
                data_type: 0x15,
                payload: vec![0x0B, 0x77, 0xAA, 0xBB],
            })
        );
    }

    #[test]
    fn resyncs_after_garbage() {
        let mut parser = SpdifParser::new();
        let bytes = [
            0x00, 0x11, 0x22, 0x72, 0xF8, 0x1F, 0x4E, 0x01, 0x00, 0x02, 0x00, 0xAB, 0xCD,
        ];
        parser.push_bytes(&bytes);
        assert_eq!(
            parser.get_next_packet(),
            Some(Iec61937Packet {
                data_type: 0x01,
                payload: vec![0xAB, 0xCD],
            })
        );
    }
}
