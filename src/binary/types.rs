use crate::error::{TdmsError, Result};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DataType {
    Void = 0x00,
    I8 = 0x01,
    I16 = 0x02,
    I32 = 0x03,
    I64 = 0x04,
    U8 = 0x05,
    U16 = 0x06,
    U32 = 0x07,
    U64 = 0x08,
    SingleFloat = 0x09,
    DoubleFloat = 0x0A,
    ExtendedFloat = 0x0B,
    SingleFloatComplex = 0x0001_0009,
    DoubleFloatComplex = 0x0001_000A,
    ExtendedFloatComplex = 0x0001_000B,
    String = 0x20,
    Boolean = 0x21,
    Timestamp = 0x44,
    FixedPoint = 0x4F,
    DAQmxRawData = 0xFFFF_FFFF,
}

impl DataType {
    pub fn from_u32(value: u32) -> Result<Self> {
        match value {
            0x00 => Ok(DataType::Void),
            0x01 => Ok(DataType::I8),
            0x02 => Ok(DataType::I16),
            0x03 => Ok(DataType::I32),
            0x04 => Ok(DataType::I64),
            0x05 => Ok(DataType::U8),
            0x06 => Ok(DataType::U16),
            0x07 => Ok(DataType::U32),
            0x08 => Ok(DataType::U64),
            0x09 | 0x19 => Ok(DataType::SingleFloat),
            0x0A | 0x1A => Ok(DataType::DoubleFloat),
            0x0B | 0x1B => Ok(DataType::ExtendedFloat),
            0x0001_0009 => Ok(DataType::SingleFloatComplex),
            0x0001_000A => Ok(DataType::DoubleFloatComplex),
            0x0001_000B => Ok(DataType::ExtendedFloatComplex),
            0x20 => Ok(DataType::String),
            0x21 => Ok(DataType::Boolean),
            0x44 => Ok(DataType::Timestamp),
            0x4F => Ok(DataType::FixedPoint),
            0xFFFF_FFFF => Ok(DataType::DAQmxRawData),
            _ => Err(TdmsError::InvalidDataType(value)),
        }
    }

    /// Returns fixed element size in bytes if fixed-size, or None for dynamic types (like String).
    pub fn element_size(&self) -> Option<usize> {
        match self {
            DataType::Void => Some(0),
            DataType::I8 | DataType::U8 | DataType::Boolean => Some(1),
            DataType::I16 | DataType::U16 => Some(2),
            DataType::I32 | DataType::U32 | DataType::SingleFloat => Some(4),
            DataType::I64 | DataType::U64 | DataType::DoubleFloat | DataType::SingleFloatComplex => Some(8),
            DataType::ExtendedFloat | DataType::DoubleFloatComplex => Some(16),
            DataType::Timestamp => Some(16),
            DataType::ExtendedFloatComplex => Some(32),
            DataType::String | DataType::FixedPoint | DataType::DAQmxRawData => None,
        }
    }
}

/// National Instruments 128-bit timestamp (64-bit seconds since 1904-01-01 + 64-bit fractional seconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct TdmsTimestamp {
    pub fraction: u64,
    pub seconds: i64,
}

impl TdmsTimestamp {
    // Seconds between 1904-01-01 (NI Epoch) and 1970-01-01 (Unix Epoch)
    pub const NI_TO_UNIX_SECONDS: i64 = 2_082_844_800;

    pub fn new(seconds: i64, fraction: u64) -> Self {
        Self { seconds, fraction }
    }

    /// Convert to Unix seconds (seconds since 1970-01-01)
    pub fn unix_seconds(&self) -> i64 {
        self.seconds - Self::NI_TO_UNIX_SECONDS
    }

    /// Convert fractional part to nanoseconds
    pub fn nanoseconds(&self) -> u32 {
        ((self.fraction as u128 * 1_000_000_000) >> 64) as u32
    }
}

impl fmt::Display for TdmsTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TdmsTimestamp(unix_sec={}, sub_nano={})",
            self.unix_seconds(),
            self.nanoseconds()
        )
    }
}
