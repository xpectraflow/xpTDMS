use crate::error::{TdmsError, Result};
use byteorder::{BigEndian, LittleEndian, ByteOrder};

pub const TDMS_LEAD_IN: &[u8; 4] = b"TDSm";
pub const HEADER_SIZE: usize = 28;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentHeader {
    pub toc_flags: u32,
    pub version: u32,
    pub next_segment_offset: u64,
    pub raw_data_offset: u64,
    pub has_metadata: bool,
    pub has_raw_data: bool,
    pub has_daqmx: bool,
    pub is_interleaved: bool,
    pub is_big_endian: bool,
    pub is_new_object_list: bool,
}

impl SegmentHeader {
    pub fn parse(bytes: &[u8; HEADER_SIZE]) -> Result<Self> {
        if &bytes[0..4] != TDMS_LEAD_IN {
            let mut lead_in = [0u8; 4];
            lead_in.copy_from_slice(&bytes[0..4]);
            return Err(TdmsError::InvalidLeadIn(lead_in));
        }

        let toc_flags = LittleEndian::read_u32(&bytes[4..8]);
        let version = LittleEndian::read_u32(&bytes[8..12]);

        let is_big_endian = (toc_flags & (1 << 6)) != 0;

        let (next_segment_offset, raw_data_offset) = if is_big_endian {
            (
                BigEndian::read_u64(&bytes[12..20]),
                BigEndian::read_u64(&bytes[20..28]),
            )
        } else {
            (
                LittleEndian::read_u64(&bytes[12..20]),
                LittleEndian::read_u64(&bytes[20..28]),
            )
        };

        Ok(Self {
            toc_flags,
            version,
            next_segment_offset,
            raw_data_offset,
            has_metadata: (toc_flags & (1 << 1)) != 0,
            is_new_object_list: (toc_flags & (1 << 2)) != 0,
            has_raw_data: (toc_flags & (1 << 3)) != 0,
            is_interleaved: (toc_flags & (1 << 5)) != 0,
            is_big_endian,
            has_daqmx: (toc_flags & (1 << 7)) != 0,
        })
    }
}
