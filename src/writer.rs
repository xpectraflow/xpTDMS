use crate::error::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct TdmsWriter {
    file: File,
}

impl TdmsWriter {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::create(path)?;
        Ok(Self { file })
    }

    /// Write a single consolidated segment containing group, channel metadata, and raw numeric channel data.
    pub fn write_channel<T: crate::model::data::TdmsPrimitive>(
        &mut self,
        group_name: &str,
        channel_name: &str,
        data: &[T],
    ) -> Result<()> {
        let mut buffer = Vec::new();

        // Lead-in Header (28 bytes)
        buffer.extend_from_slice(b"TDSm");
        buffer.extend_from_slice(&(0x0eu32).to_le_bytes()); // TOC: Has Metadata + Has Raw Data
        buffer.extend_from_slice(&(4713u32).to_le_bytes());

        let header_offsets_pos = buffer.len();
        buffer.extend_from_slice(&(0u64).to_le_bytes()); // Next segment offset placeholder
        buffer.extend_from_slice(&(0u64).to_le_bytes()); // Raw data offset placeholder

        let metadata_start = buffer.len();

        // 3 Objects: "/", "/'Group'", "/'Group'/'Channel'"
        buffer.extend_from_slice(&(3u32).to_le_bytes());

        // Root "/"
        let root_path = "/";
        buffer.extend_from_slice(&(root_path.len() as u32).to_le_bytes());
        buffer.extend_from_slice(root_path.as_bytes());
        buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
        buffer.extend_from_slice(&(0u32).to_le_bytes());

        // Group
        let group_path = format!("/'{}'", group_name);
        buffer.extend_from_slice(&(group_path.len() as u32).to_le_bytes());
        buffer.extend_from_slice(group_path.as_bytes());
        buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
        buffer.extend_from_slice(&(0u32).to_le_bytes());

        // Channel
        let chan_path = format!("/'{}'/'{}'", group_name, channel_name);
        buffer.extend_from_slice(&(chan_path.len() as u32).to_le_bytes());
        buffer.extend_from_slice(chan_path.as_bytes());
        buffer.extend_from_slice(&(16u32).to_le_bytes());
        buffer.extend_from_slice(&(T::data_type() as u32).to_le_bytes());
        buffer.extend_from_slice(&(1u32).to_le_bytes());
        buffer.extend_from_slice(&(data.len() as u64).to_le_bytes());
        buffer.extend_from_slice(&(0u32).to_le_bytes());

        let raw_data_start = buffer.len();
        let raw_data_offset = (raw_data_start - metadata_start) as u64;

        // Raw data unsafe slice copy
        let raw_bytes = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * std::mem::size_of::<T>(),
            )
        };
        buffer.extend_from_slice(raw_bytes);

        let total_size = buffer.len() as u64;
        let next_segment_offset = total_size - 28;

        buffer[header_offsets_pos..header_offsets_pos + 8].copy_from_slice(&next_segment_offset.to_le_bytes());
        buffer[header_offsets_pos + 8..header_offsets_pos + 16].copy_from_slice(&raw_data_offset.to_le_bytes());

        self.file.write_all(&buffer)?;
        self.file.flush()?;
        Ok(())
    }
}
