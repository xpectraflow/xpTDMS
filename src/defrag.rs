use crate::binary::{DataType, TdmsTimestamp};
use crate::error::Result;
use crate::model::file::TdmsFile;
use crate::model::property::PropertyValue;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn write_property_value(buffer: &mut Vec<u8>, val: &PropertyValue) {
    buffer.extend_from_slice(&(val.data_type() as u32).to_le_bytes());
    match val {
        PropertyValue::Void => {}
        PropertyValue::I8(v) => buffer.push(*v as u8),
        PropertyValue::I16(v) => buffer.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::I32(v) => buffer.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::I64(v) => buffer.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::U8(v) => buffer.push(*v),
        PropertyValue::U16(v) => buffer.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::U32(v) => buffer.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::U64(v) => buffer.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::SingleFloat(v) => buffer.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::DoubleFloat(v) => buffer.extend_from_slice(&v.to_le_bytes()),
        PropertyValue::String(s) => {
            buffer.extend_from_slice(&(s.len() as u32).to_le_bytes());
            buffer.extend_from_slice(s.as_bytes());
        }
        PropertyValue::Boolean(b) => buffer.push(if *b { 1 } else { 0 }),
        PropertyValue::Timestamp(ts) => {
            buffer.extend_from_slice(&ts.fraction.to_le_bytes());
            buffer.extend_from_slice(&ts.seconds.to_le_bytes());
        }
    }
}

fn write_properties(buffer: &mut Vec<u8>, props: &HashMap<String, PropertyValue>) {
    buffer.extend_from_slice(&(props.len() as u32).to_le_bytes());
    for (name, val) in props {
        buffer.extend_from_slice(&(name.len() as u32).to_le_bytes());
        buffer.extend_from_slice(name.as_bytes());
        write_property_value(buffer, val);
    }
}

pub struct Defragmenter;

impl Defragmenter {
    /// Defragment a TDMS file, writing a consolidated and optimized version to `output_path`.
    pub fn defragment<P: AsRef<Path>, Q: AsRef<Path>>(input_path: P, output_path: Q) -> Result<()> {
        let tdms_file = TdmsFile::open(input_path)?;
        let mut out_file = File::create(output_path)?;

        let mut buffer = Vec::new();

        // 1. Header & Lead-in
        buffer.extend_from_slice(b"TDSm");
        buffer.extend_from_slice(&(0x0eu32).to_le_bytes()); // TOC: Has Metadata + Has Raw Data
        buffer.extend_from_slice(&(4713u32).to_le_bytes()); // TDMS 2.0 version

        let header_offsets_pos = buffer.len();
        buffer.extend_from_slice(&(0u64).to_le_bytes()); // Placeholder: next segment offset
        buffer.extend_from_slice(&(0u64).to_le_bytes()); // Placeholder: raw data offset

        let metadata_start = buffer.len();

        // Count objects
        let mut total_objects = 1; // Root
        for group in tdms_file.groups.values() {
            total_objects += 1;
            total_objects += group.channels.len();
        }

        buffer.extend_from_slice(&(total_objects as u32).to_le_bytes());

        // Object 1: Root "/"
        buffer.extend_from_slice(&(1u32).to_le_bytes());
        buffer.extend_from_slice(b"/");
        buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes()); // No raw data
        write_properties(&mut buffer, &tdms_file.properties);

        // Object Groups & Channels
        for group in tdms_file.groups.values() {
            buffer.extend_from_slice(&(group.path.len() as u32).to_le_bytes());
            buffer.extend_from_slice(group.path.as_bytes());
            buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
            write_properties(&mut buffer, &group.properties);

            for channel in group.channels.values() {
                buffer.extend_from_slice(&(channel.path.len() as u32).to_le_bytes());
                buffer.extend_from_slice(channel.path.as_bytes());

                if let Some(dtype) = channel.data_type {
                    buffer.extend_from_slice(&(16u32).to_le_bytes());
                    buffer.extend_from_slice(&(dtype as u32).to_le_bytes());
                    buffer.extend_from_slice(&(1u32).to_le_bytes());
                    buffer.extend_from_slice(&(channel.number_of_values as u64).to_le_bytes());
                } else {
                    buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
                }

                write_properties(&mut buffer, &channel.properties);
            }
        }

        let raw_data_start = buffer.len();
        let raw_data_offset = (raw_data_start - metadata_start) as u64;

        // Copy consolidated raw data blocks
        for group in tdms_file.groups.values() {
            for channel in group.channels.values() {
                if let Some(dtype) = channel.data_type {
                    match dtype {
                        DataType::DoubleFloat => {
                            if let Ok(data) = tdms_file.read_channel_data::<f64>(&group.name, &channel.name) {
                                for v in data { buffer.extend_from_slice(&v.to_le_bytes()); }
                            }
                        }
                        DataType::SingleFloat => {
                            if let Ok(data) = tdms_file.read_channel_data::<f32>(&group.name, &channel.name) {
                                for v in data { buffer.extend_from_slice(&v.to_le_bytes()); }
                            }
                        }
                        DataType::I8 => {
                            if let Ok(data) = tdms_file.read_channel_data::<i8>(&group.name, &channel.name) {
                                for v in data { buffer.push(v as u8); }
                            }
                        }
                        DataType::I16 => {
                            if let Ok(data) = tdms_file.read_channel_data::<i16>(&group.name, &channel.name) {
                                for v in data { buffer.extend_from_slice(&v.to_le_bytes()); }
                            }
                        }
                        DataType::I32 => {
                            if let Ok(data) = tdms_file.read_channel_data::<i32>(&group.name, &channel.name) {
                                for v in data { buffer.extend_from_slice(&v.to_le_bytes()); }
                            }
                        }
                        DataType::I64 => {
                            if let Ok(data) = tdms_file.read_channel_data::<i64>(&group.name, &channel.name) {
                                for v in data { buffer.extend_from_slice(&v.to_le_bytes()); }
                            }
                        }
                        DataType::U8 => {
                            if let Ok(data) = tdms_file.read_channel_data::<u8>(&group.name, &channel.name) {
                                for v in data { buffer.push(v); }
                            }
                        }
                        DataType::U16 => {
                            if let Ok(data) = tdms_file.read_channel_data::<u16>(&group.name, &channel.name) {
                                for v in data { buffer.extend_from_slice(&v.to_le_bytes()); }
                            }
                        }
                        DataType::U32 => {
                            if let Ok(data) = tdms_file.read_channel_data::<u32>(&group.name, &channel.name) {
                                for v in data { buffer.extend_from_slice(&v.to_le_bytes()); }
                            }
                        }
                        DataType::U64 => {
                            if let Ok(data) = tdms_file.read_channel_data::<u64>(&group.name, &channel.name) {
                                for v in data { buffer.extend_from_slice(&v.to_le_bytes()); }
                            }
                        }
                        DataType::Boolean => {
                            if let Ok(data) = tdms_file.read_channel_data::<bool>(&group.name, &channel.name) {
                                for v in data { buffer.push(if v { 1 } else { 0 }); }
                            }
                        }
                        DataType::Timestamp => {
                            if let Ok(data) = tdms_file.read_channel_data::<TdmsTimestamp>(&group.name, &channel.name) {
                                for v in data {
                                    buffer.extend_from_slice(&v.fraction.to_le_bytes());
                                    buffer.extend_from_slice(&v.seconds.to_le_bytes());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let total_size = buffer.len() as u64;
        let next_segment_offset = total_size - 28;

        buffer[header_offsets_pos..header_offsets_pos + 8].copy_from_slice(&next_segment_offset.to_le_bytes());
        buffer[header_offsets_pos + 8..header_offsets_pos + 16].copy_from_slice(&raw_data_offset.to_le_bytes());

        out_file.write_all(&buffer)?;
        Ok(())
    }
}
