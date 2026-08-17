use memmap2::MmapMut;
use xpTDMS::{ChunkIterator, DataType, TdmsFile};

#[test]
fn test_synthetic_tdms_parsing_and_channel_reading() {
    let mut buffer = Vec::new();

    // --- Segment 1 Header (28 bytes) ---
    buffer.extend_from_slice(b"TDSm");
    buffer.extend_from_slice(&(0x0eu32).to_le_bytes()); // TOC: has metadata (0x02) + has raw data (0x08) + Little Endian
    buffer.extend_from_slice(&(4713u32).to_le_bytes()); // Version

    let header_len_pos = buffer.len();
    buffer.extend_from_slice(&(0u64).to_le_bytes()); // Placeholder: next segment offset
    buffer.extend_from_slice(&(0u64).to_le_bytes()); // Placeholder: raw data offset

    let metadata_start = buffer.len();

    // --- Metadata Block ---
    buffer.extend_from_slice(&(3u32).to_le_bytes()); // 3 objects

    // Object 1: Root "/"
    let root_path = "/";
    buffer.extend_from_slice(&(root_path.len() as u32).to_le_bytes());
    buffer.extend_from_slice(root_path.as_bytes());
    buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes()); // No raw data
    buffer.extend_from_slice(&(1u32).to_le_bytes()); // 1 property
    let prop_name = "author";
    buffer.extend_from_slice(&(prop_name.len() as u32).to_le_bytes());
    buffer.extend_from_slice(prop_name.as_bytes());
    buffer.extend_from_slice(&(DataType::String as u32).to_le_bytes());
    let prop_val = "xpTDMS Developer";
    buffer.extend_from_slice(&(prop_val.len() as u32).to_le_bytes());
    buffer.extend_from_slice(prop_val.as_bytes());

    // Object 2: Group "/\"Group1\""
    let group_path = "/\"Group1\"";
    buffer.extend_from_slice(&(group_path.len() as u32).to_le_bytes());
    buffer.extend_from_slice(group_path.as_bytes());
    buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    // Object 3: Channel "/\"Group1\"/\"Channel1\""
    let chan_path = "/\"Group1\"/\"Channel1\"";
    buffer.extend_from_slice(&(chan_path.len() as u32).to_le_bytes());
    buffer.extend_from_slice(chan_path.as_bytes());
    buffer.extend_from_slice(&(16u32).to_le_bytes()); // Raw data header length (16 bytes: dtype + dim + count)
    buffer.extend_from_slice(&(DataType::DoubleFloat as u32).to_le_bytes());
    buffer.extend_from_slice(&(1u32).to_le_bytes());
    buffer.extend_from_slice(&(100u64).to_le_bytes()); // 100 values
    buffer.extend_from_slice(&(0u32).to_le_bytes()); // 0 properties

    let raw_data_start = buffer.len();
    let raw_data_offset = (raw_data_start - metadata_start) as u64;

    // --- Raw Data Block (100 * 8 bytes = 800 bytes) ---
    let mut expected_values = Vec::with_capacity(100);
    for i in 0..100 {
        let val = i as f64 * 1.5;
        expected_values.push(val);
        buffer.extend_from_slice(&val.to_le_bytes());
    }

    let total_len = buffer.len();
    let next_segment_offset = (total_len - 28) as u64;

    buffer[header_len_pos..header_len_pos + 8].copy_from_slice(&next_segment_offset.to_le_bytes());
    buffer[header_len_pos + 8..header_len_pos + 16].copy_from_slice(&raw_data_offset.to_le_bytes());

    let mut mmap_mut = MmapMut::map_anon(buffer.len()).unwrap();
    mmap_mut.copy_from_slice(&buffer);
    let mmap = mmap_mut.make_read_only().unwrap();

    let tdms_file = TdmsFile::from_mmap(mmap).expect("Failed to parse synthetic TDMS");

    // Read full raw channel data
    let channel_data = tdms_file
        .read_channel_data::<f64>("Group1", "Channel1")
        .expect("Should read f64 channel data");

    assert_eq!(channel_data.len(), 100);
    assert_eq!(channel_data, expected_values);

    // Test Chunk Streaming (chunks of size 30)
    let chunks: Vec<&[f64]> = ChunkIterator::new(&channel_data, 30).collect();
    assert_eq!(chunks.len(), 4); // 30 + 30 + 30 + 10
    assert_eq!(chunks[0].len(), 30);
    assert_eq!(chunks[1].len(), 30);
    assert_eq!(chunks[2].len(), 30);
    assert_eq!(chunks[3].len(), 10);
    assert_eq!(chunks[0], &expected_values[0..30]);
}

#[test]
fn test_multi_channel_same_segment() {
    let mut buffer = Vec::new();

    // Lead-in Header (28 bytes)
    buffer.extend_from_slice(b"TDSm");
    buffer.extend_from_slice(&(0x0eu32).to_le_bytes()); // TOC: Metadata + Raw Data
    buffer.extend_from_slice(&(4713u32).to_le_bytes());

    let header_len_pos = buffer.len();
    buffer.extend_from_slice(&(0u64).to_le_bytes());
    buffer.extend_from_slice(&(0u64).to_le_bytes());

    let metadata_start = buffer.len();

    // 4 Objects: Root, Group, Chan1, Chan2
    buffer.extend_from_slice(&(4u32).to_le_bytes());

    // Root
    let root = "/";
    buffer.extend_from_slice(&(root.len() as u32).to_le_bytes());
    buffer.extend_from_slice(root.as_bytes());
    buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    // Group
    let group = "/'Sensors'";
    buffer.extend_from_slice(&(group.len() as u32).to_le_bytes());
    buffer.extend_from_slice(group.as_bytes());
    buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    // Chan1 (100 values of f64)
    let chan1 = "/'Sensors'/'Temp1'";
    buffer.extend_from_slice(&(chan1.len() as u32).to_le_bytes());
    buffer.extend_from_slice(chan1.as_bytes());
    buffer.extend_from_slice(&(16u32).to_le_bytes());
    buffer.extend_from_slice(&(DataType::DoubleFloat as u32).to_le_bytes());
    buffer.extend_from_slice(&(1u32).to_le_bytes());
    buffer.extend_from_slice(&(100u64).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    // Chan2 (50 values of f64)
    let chan2 = "/'Sensors'/'Temp2'";
    buffer.extend_from_slice(&(chan2.len() as u32).to_le_bytes());
    buffer.extend_from_slice(chan2.as_bytes());
    buffer.extend_from_slice(&(16u32).to_le_bytes());
    buffer.extend_from_slice(&(DataType::DoubleFloat as u32).to_le_bytes());
    buffer.extend_from_slice(&(1u32).to_le_bytes());
    buffer.extend_from_slice(&(50u64).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    let raw_data_start = buffer.len();
    let raw_data_offset = (raw_data_start - metadata_start) as u64;

    // Raw Data Block
    let chan1_expected: Vec<f64> = (0..100).map(|i| 20.0 + i as f64 * 0.1).collect();
    for &v in &chan1_expected {
        buffer.extend_from_slice(&v.to_le_bytes());
    }

    let chan2_expected: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 0.5).collect();
    for &v in &chan2_expected {
        buffer.extend_from_slice(&v.to_le_bytes());
    }

    let total_len = buffer.len();
    let next_segment_offset = (total_len - 28) as u64;
    buffer[header_len_pos..header_len_pos + 8].copy_from_slice(&next_segment_offset.to_le_bytes());
    buffer[header_len_pos + 8..header_len_pos + 16].copy_from_slice(&raw_data_offset.to_le_bytes());

    let mut mmap_mut = MmapMut::map_anon(buffer.len()).unwrap();
    mmap_mut.copy_from_slice(&buffer);
    let mmap = mmap_mut.make_read_only().unwrap();

    let tdms_file = TdmsFile::from_mmap(mmap).expect("Should parse multi-channel segment");
    let c1 = tdms_file.read_channel_data::<f64>("Sensors", "Temp1").unwrap();
    let c2 = tdms_file.read_channel_data::<f64>("Sensors", "Temp2").unwrap();

    assert_eq!(c1, chan1_expected);
    assert_eq!(c2, chan2_expected);
}

#[test]
fn test_raw_data_index_inheritance() {
    let mut buffer = Vec::new();

    // Segment 1 Header
    buffer.extend_from_slice(b"TDSm");
    buffer.extend_from_slice(&(0x0eu32).to_le_bytes());
    buffer.extend_from_slice(&(4713u32).to_le_bytes());
    let h1_offset_pos = buffer.len();
    buffer.extend_from_slice(&(0u64).to_le_bytes());
    buffer.extend_from_slice(&(0u64).to_le_bytes());

    let m1_start = buffer.len();
    buffer.extend_from_slice(&(2u32).to_le_bytes()); // 2 objects: Root, Chan

    let root = "/";
    buffer.extend_from_slice(&(root.len() as u32).to_le_bytes());
    buffer.extend_from_slice(root.as_bytes());
    buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    let chan = "/'Group'/'Chan'";
    buffer.extend_from_slice(&(chan.len() as u32).to_le_bytes());
    buffer.extend_from_slice(chan.as_bytes());
    buffer.extend_from_slice(&(16u32).to_le_bytes());
    buffer.extend_from_slice(&(DataType::DoubleFloat as u32).to_le_bytes());
    buffer.extend_from_slice(&(1u32).to_le_bytes());
    buffer.extend_from_slice(&(50u64).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    let r1_start = buffer.len();
    let r1_offset = (r1_start - m1_start) as u64;
    for i in 0..50 {
        buffer.extend_from_slice(&(i as f64).to_le_bytes());
    }
    let s1_len = buffer.len();
    let next1 = (s1_len - 28) as u64;
    buffer[h1_offset_pos..h1_offset_pos + 8].copy_from_slice(&next1.to_le_bytes());
    buffer[h1_offset_pos + 8..h1_offset_pos + 16].copy_from_slice(&r1_offset.to_le_bytes());

    // Segment 2 Header (Reuses index layout via 0x0000_0000)
    let s2_start = buffer.len();
    buffer.extend_from_slice(b"TDSm");
    buffer.extend_from_slice(&(0x0eu32).to_le_bytes());
    buffer.extend_from_slice(&(4713u32).to_le_bytes());
    let h2_offset_pos = buffer.len();
    buffer.extend_from_slice(&(0u64).to_le_bytes());
    buffer.extend_from_slice(&(0u64).to_le_bytes());

    let m2_start = buffer.len();
    buffer.extend_from_slice(&(1u32).to_le_bytes()); // 1 object: Chan with 0x0000_0000 index
    buffer.extend_from_slice(&(chan.len() as u32).to_le_bytes());
    buffer.extend_from_slice(chan.as_bytes());
    buffer.extend_from_slice(&(0x0000_0000u32).to_le_bytes()); // Reuses previous segment raw data index!
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    let r2_start = buffer.len();
    let r2_offset = (r2_start - m2_start) as u64;
    for i in 50..100 {
        buffer.extend_from_slice(&(i as f64).to_le_bytes());
    }
    let s2_len = buffer.len() - s2_start;
    let next2 = (s2_len - 28) as u64;
    buffer[h2_offset_pos..h2_offset_pos + 8].copy_from_slice(&next2.to_le_bytes());
    buffer[h2_offset_pos + 8..h2_offset_pos + 16].copy_from_slice(&r2_offset.to_le_bytes());

    let mut mmap_mut = MmapMut::map_anon(buffer.len()).unwrap();
    mmap_mut.copy_from_slice(&buffer);
    let mmap = mmap_mut.make_read_only().unwrap();

    let tdms_file = TdmsFile::from_mmap(mmap).expect("Should parse multi-segment file with raw data index inheritance");
    let data = tdms_file.read_channel_data::<f64>("Group", "Chan").unwrap();

    assert_eq!(data.len(), 100);
    let expected: Vec<f64> = (0..100).map(|i| i as f64).collect();
    assert_eq!(data, expected);
}

#[test]
fn test_interleaved_raw_data() {
    let mut buffer = Vec::new();

    // Lead-in Header: ToC flags = 0x2E (metadata + raw data + interleaved bit 0x20)
    buffer.extend_from_slice(b"TDSm");
    buffer.extend_from_slice(&(0x2eu32).to_le_bytes());
    buffer.extend_from_slice(&(4713u32).to_le_bytes());
    let h_offset_pos = buffer.len();
    buffer.extend_from_slice(&(0u64).to_le_bytes());
    buffer.extend_from_slice(&(0u64).to_le_bytes());

    let m_start = buffer.len();
    buffer.extend_from_slice(&(3u32).to_le_bytes()); // Root, Chan1, Chan2

    let root = "/";
    buffer.extend_from_slice(&(root.len() as u32).to_le_bytes());
    buffer.extend_from_slice(root.as_bytes());
    buffer.extend_from_slice(&(0xFFFF_FFFFu32).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    let c1_path = "/'G'/'C1'";
    buffer.extend_from_slice(&(c1_path.len() as u32).to_le_bytes());
    buffer.extend_from_slice(c1_path.as_bytes());
    buffer.extend_from_slice(&(16u32).to_le_bytes());
    buffer.extend_from_slice(&(DataType::DoubleFloat as u32).to_le_bytes());
    buffer.extend_from_slice(&(1u32).to_le_bytes());
    buffer.extend_from_slice(&(3u64).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    let c2_path = "/'G'/'C2'";
    buffer.extend_from_slice(&(c2_path.len() as u32).to_le_bytes());
    buffer.extend_from_slice(c2_path.as_bytes());
    buffer.extend_from_slice(&(16u32).to_le_bytes());
    buffer.extend_from_slice(&(DataType::DoubleFloat as u32).to_le_bytes());
    buffer.extend_from_slice(&(1u32).to_le_bytes());
    buffer.extend_from_slice(&(3u64).to_le_bytes());
    buffer.extend_from_slice(&(0u32).to_le_bytes());

    let r_start = buffer.len();
    let r_offset = (r_start - m_start) as u64;

    // Interleaved data: [C1_s1, C2_s1, C1_s2, C2_s2, C1_s3, C2_s3]
    let interleaved_raw: Vec<f64> = vec![1.0, 10.0, 2.0, 20.0, 3.0, 30.0];
    for &v in &interleaved_raw {
        buffer.extend_from_slice(&v.to_le_bytes());
    }

    let tot_len = buffer.len();
    let next_off = (tot_len - 28) as u64;
    buffer[h_offset_pos..h_offset_pos + 8].copy_from_slice(&next_off.to_le_bytes());
    buffer[h_offset_pos + 8..h_offset_pos + 16].copy_from_slice(&r_offset.to_le_bytes());

    let mut mmap_mut = MmapMut::map_anon(buffer.len()).unwrap();
    mmap_mut.copy_from_slice(&buffer);
    let mmap = mmap_mut.make_read_only().unwrap();

    let tdms = TdmsFile::from_mmap(mmap).expect("Should parse interleaved TDMS segment");
    let c1_data = tdms.read_channel_data::<f64>("G", "C1").unwrap();
    let c2_data = tdms.read_channel_data::<f64>("G", "C2").unwrap();

    assert_eq!(c1_data, vec![1.0, 2.0, 3.0]);
    assert_eq!(c2_data, vec![10.0, 20.0, 30.0]);
}
