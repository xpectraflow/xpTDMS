use crate::binary::{SegmentHeader, SliceReader, HEADER_SIZE};
use crate::error::{TdmsError, Result};
use crate::index::SegmentIndex;
use crate::model::channel::TdmsChannel;
use crate::model::data::TdmsPrimitive;
use crate::model::group::TdmsGroup;
use crate::model::property::PropertyValue;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[derive(Debug)]
pub struct TdmsFile {
    pub properties: HashMap<String, PropertyValue>,
    pub groups: HashMap<String, TdmsGroup>,
    pub segments: Vec<SegmentIndex>,
    mmap: Mmap,
}

pub fn parse_object_path(path: &str) -> (Option<String>, Option<String>) {
    let clean = path.trim();
    if clean == "/" || clean == "'/'" || clean.is_empty() {
        return (None, None);
    }
    let parts: Vec<String> = clean
        .split('/')
        .map(|s| s.trim_matches('\'').trim_matches('"').trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() == 1 {
        (Some(parts[0].clone()), None)
    } else if parts.len() >= 2 {
        (Some(parts[0].clone()), Some(parts[1].clone()))
    } else {
        (None, None)
    }
}

impl TdmsFile {
    /// Open and memory-map a TDMS file from disk, indexing all segments in zero-copy mode.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Self::from_mmap(mmap)
    }

    /// Parse TDMS file directly from an existing memory map.
    pub fn from_mmap(mmap: Mmap) -> Result<Self> {
        let total_size = mmap.len() as u64;
        let mut offset = 0u64;
        let mut segments = Vec::new();

        let mut root_properties = HashMap::new();
        let mut groups: HashMap<String, TdmsGroup> = HashMap::new();
        let mut prev_index_map = HashMap::new();

        while offset + HEADER_SIZE as u64 <= total_size {
            let mut header_bytes = [0u8; HEADER_SIZE];
            header_bytes.copy_from_slice(&mmap[offset as usize..offset as usize + HEADER_SIZE]);

            let header = match SegmentHeader::parse(&header_bytes) {
                Ok(h) => h,
                Err(e) => {
                    if offset == 0 {
                        return Err(e);
                    } else {
                        break;
                    }
                }
            };

            let metadata_slice_start = (offset + HEADER_SIZE as u64) as usize;
            let metadata_slice_end = metadata_slice_start + header.raw_data_offset as usize;

            if metadata_slice_end > mmap.len() {
                return Err(TdmsError::UnexpectedEof {
                    required: metadata_slice_end,
                    available: mmap.len(),
                });
            }

            let metadata_slice = &mmap[metadata_slice_start..metadata_slice_end];
            let mut reader = SliceReader::new(metadata_slice, header.is_big_endian);

            let segment_index = SegmentIndex::parse_metadata(&mut reader, &header, offset, total_size, &mut prev_index_map)?;

            // Update Object Hierarchy & Properties
            for obj in &segment_index.objects {
                let (group_opt, chan_opt) = parse_object_path(&obj.path);
                match (group_opt, chan_opt) {
                    (None, None) => {
                        root_properties.extend(obj.properties.clone());
                    }
                    (Some(group_name), None) => {
                        let group = groups
                            .entry(group_name.clone())
                            .or_insert_with(|| TdmsGroup::new(group_name, obj.path.clone()));
                        group.properties.extend(obj.properties.clone());
                    }
                    (Some(group_name), Some(channel_name)) => {
                        let group = groups
                            .entry(group_name.clone())
                            .or_insert_with(|| TdmsGroup::new(group_name.clone(), format!("/'{}'", group_name)));

                        let channel = group
                            .channels
                            .entry(channel_name.clone())
                            .or_insert_with(|| TdmsChannel::new(obj.path.clone(), group_name.clone(), channel_name));

                        channel.properties.extend(obj.properties.clone());

                        if let Some(ref raw_idx) = obj.raw_data_index {
                            channel.data_type = Some(raw_idx.data_type);
                            channel.number_of_values += raw_idx.number_of_values;
                        }
                    }
                    _ => {}
                }
            }

            offset += segment_index.segment_total_size;
            if segment_index.segment_total_size == 0 {
                break;
            }
            segments.push(segment_index);
        }

        Ok(Self {
            properties: root_properties,
            groups,
            segments,
            mmap,
        })
    }

    pub fn group(&self, name: &str) -> Option<&TdmsGroup> {
        self.groups.get(name)
    }

    pub fn channel(&self, group_name: &str, channel_name: &str) -> Option<&TdmsChannel> {
        self.group(group_name)?.channel(channel_name)
    }

    pub fn mmap(&self) -> &Mmap {
        &self.mmap
    }

    /// Read raw data for a specific channel as a vector of typed primitives.
    pub fn read_channel_data<T: TdmsPrimitive>(&self, group_name: &str, channel_name: &str) -> Result<Vec<T>> {
        let channel = self
            .channel(group_name, channel_name)
            .ok_or_else(|| TdmsError::CorruptedMetadata {
                offset: 0,
                reason: format!("Channel '/\"{}\"/\"{}\"' not found", group_name, channel_name),
            })?;

        let expected_dtype = T::data_type();
        if let Some(dtype) = channel.data_type {
            if dtype != expected_dtype {
                return Err(TdmsError::CorruptedMetadata {
                    offset: 0,
                    reason: format!("Channel data type mismatch: expected {:?}, found {:?}", expected_dtype, dtype),
                });
            }
        }

        let mut data = Vec::with_capacity(channel.number_of_values as usize);

        for segment in &self.segments {
            if !segment.header.has_raw_data {
                continue;
            }

            let raw_start = segment.raw_data_offset as usize;
            let raw_end = (segment.header_offset + segment.segment_total_size) as usize;

            if raw_start >= raw_end || raw_end > self.mmap.len() {
                continue;
            }

            let raw_slice = &self.mmap[raw_start..raw_end];

            if segment.header.is_interleaved {
                let mut frame_size = 0usize;
                let mut target_offset_in_frame = 0usize;
                let mut target_elem_size = 0usize;
                let mut found = false;

                for obj in &segment.objects {
                    if let Some(ref idx) = obj.raw_data_index {
                        let elem_size = idx.data_type.element_size().unwrap_or(0);
                        let (g_opt, c_opt) = parse_object_path(&obj.path);
                        if let (Some(ref g), Some(ref c)) = (g_opt, c_opt) {
                            if g == group_name && c == channel_name {
                                target_offset_in_frame = frame_size;
                                target_elem_size = elem_size;
                                found = true;
                            }
                        }
                        frame_size += elem_size;
                    }
                }

                if found && frame_size > 0 && target_elem_size > 0 {
                    let total_frames = raw_slice.len() / frame_size;
                    for f in 0..total_frames {
                        let sample_start = f * frame_size + target_offset_in_frame;
                        if sample_start + target_elem_size <= raw_slice.len() {
                            let sample_slice = &raw_slice[sample_start..sample_start + target_elem_size];
                            let val = T::read_slice(sample_slice, segment.header.is_big_endian, 1)?;
                            data.extend(val);
                        }
                    }
                }
            } else {
                let mut single_chunk_size = 0usize;
                for obj in &segment.objects {
                    if let Some(ref idx) = obj.raw_data_index {
                        let size = if let Some(total_bytes) = idx.total_size_bytes {
                            total_bytes as usize
                        } else if let Some(elem_size) = idx.data_type.element_size() {
                            (idx.number_of_values as usize) * elem_size
                        } else {
                            0
                        };
                        single_chunk_size += size;
                    }
                }

                let num_chunks = if single_chunk_size > 0 {
                    (raw_slice.len() / single_chunk_size).max(1)
                } else {
                    1
                };

                for c in 0..num_chunks {
                    let chunk_start = c * single_chunk_size;
                    let mut raw_byte_offset = 0usize;

                    for obj in &segment.objects {
                        if let Some(ref idx) = obj.raw_data_index {
                            let size = if let Some(total_bytes) = idx.total_size_bytes {
                                total_bytes as usize
                            } else if let Some(elem_size) = idx.data_type.element_size() {
                                (idx.number_of_values as usize) * elem_size
                            } else {
                                0
                            };

                            let (g_opt, c_opt) = parse_object_path(&obj.path);
                            if let (Some(ref g), Some(ref c_name)) = (g_opt, c_opt) {
                                if g == group_name && c_name == channel_name {
                                    let count = idx.number_of_values as usize;
                                    let start_pos = chunk_start + raw_byte_offset;
                                    if start_pos + size <= raw_slice.len() {
                                        let obj_slice = &raw_slice[start_pos..start_pos + size];
                                        let chunk_data = T::read_slice(obj_slice, segment.header.is_big_endian, count)?;
                                        data.extend(chunk_data);
                                    }
                                }
                            }
                            raw_byte_offset += size;
                        }
                    }
                }
            }
        }

        Ok(data)
    }
}
