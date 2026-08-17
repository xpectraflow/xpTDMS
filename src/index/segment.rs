use crate::binary::{DataType, SegmentHeader, SliceReader, HEADER_SIZE};
use crate::error::Result;
use crate::model::property::PropertyValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ObjectRawDataIndex {
    pub data_type: DataType,
    pub dimension: u32,
    pub number_of_values: u64,
    pub total_size_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    pub path: String,
    pub raw_data_index: Option<ObjectRawDataIndex>,
    pub properties: HashMap<String, PropertyValue>,
}

#[derive(Debug, Clone)]
pub struct SegmentIndex {
    pub header: SegmentHeader,
    pub header_offset: u64,
    pub metadata_offset: u64,
    pub raw_data_offset: u64,
    pub segment_total_size: u64,
    pub objects: Vec<ObjectMetadata>,
}

impl SegmentIndex {
    pub fn parse_metadata(
        reader: &mut SliceReader,
        header: &SegmentHeader,
        header_offset: u64,
        total_file_size: u64,
        prev_index_map: &mut HashMap<String, ObjectRawDataIndex>,
    ) -> Result<Self> {
        let metadata_offset = header_offset + HEADER_SIZE as u64;
        let raw_data_offset = metadata_offset + header.raw_data_offset;

        let segment_total_size = if header.next_segment_offset == 0xFFFF_FFFF_FFFF_FFFF {
            total_file_size.saturating_sub(header_offset)
        } else {
            HEADER_SIZE as u64 + header.next_segment_offset
        };

        let mut objects = Vec::new();

        if header.has_metadata {
            let num_objects = reader.read_u32()?;
            for _ in 0..num_objects {
                let path = reader.read_string()?;
                let raw_data_header = reader.read_u32()?;

                let raw_data_index = match raw_data_header {
                    0xFFFF_FFFF => None, // No raw data for this object in this segment
                    0x0000_0000 => prev_index_map.get(&path).cloned(), // Reuses previous segment raw data index layout
                    _ => {
                        let dtype = reader.read_data_type()?;
                        let dimension = reader.read_u32()?;
                        let number_of_values = reader.read_u64()?;
                        let mut bytes_read = 16;
                        let total_size_bytes = if dtype == DataType::String {
                            bytes_read += 8;
                            Some(reader.read_u64()?)
                        } else {
                            None
                        };
                        let descriptor_len = raw_data_header as usize;
                        if descriptor_len > bytes_read {
                            reader.read_bytes(descriptor_len - bytes_read)?;
                        }
                        let idx = ObjectRawDataIndex {
                            data_type: dtype,
                            dimension,
                            number_of_values,
                            total_size_bytes,
                        };
                        prev_index_map.insert(path.clone(), idx.clone());
                        Some(idx)
                    }
                };

                let num_properties = reader.read_u32()?;
                let mut properties = HashMap::with_capacity(num_properties as usize);
                for _ in 0..num_properties {
                    let prop_name = reader.read_string()?;
                    let prop_val = PropertyValue::parse(reader)?;
                    properties.insert(prop_name, prop_val);
                }

                objects.push(ObjectMetadata {
                    path,
                    raw_data_index,
                    properties,
                });
            }
        }

        Ok(Self {
            header: header.clone(),
            header_offset,
            metadata_offset,
            raw_data_offset,
            segment_total_size,
            objects,
        })
    }
}
