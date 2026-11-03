//! Small audio files built in memory, so tests need no binary fixtures.

pub fn pcm_samples() -> Vec<i16> {
    (0..4410).map(|i| ((i % 100) * 100) as i16).collect()
}

pub fn wav_pcm() -> Vec<u8> {
    pcm_samples().iter().flat_map(|s| s.to_le_bytes()).collect()
}

pub fn wav_bytes() -> Vec<u8> {
    let data = wav_pcm();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&44100u32.to_le_bytes());
    bytes.extend_from_slice(&88200u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&data);
    bytes
}

pub fn aiff_pcm() -> Vec<u8> {
    pcm_samples().iter().flat_map(|s| s.to_be_bytes()).collect()
}

pub fn aiff_bytes() -> Vec<u8> {
    let data = aiff_pcm();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"FORM");
    bytes.extend_from_slice(&(4 + 26 + 16 + data.len() as u32).to_be_bytes());
    bytes.extend_from_slice(b"AIFFCOMM");
    bytes.extend_from_slice(&18u32.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes()); // mono
    bytes.extend_from_slice(&(pcm_samples().len() as u32).to_be_bytes());
    bytes.extend_from_slice(&16u16.to_be_bytes());
    // 44100 as an 80-bit extended float
    bytes.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
    bytes.extend_from_slice(b"SSND");
    bytes.extend_from_slice(&(8 + data.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&[0; 8]); // offset and block size
    bytes.extend_from_slice(&data);
    bytes
}

/// Silent MPEG-1 Layer III frames: 128 kbps, 44.1 kHz, no padding.
pub fn mp3_bytes() -> Vec<u8> {
    let mut frame = vec![0u8; 417];
    frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
    frame.repeat(40)
}

pub fn flac_audio() -> Vec<u8> {
    (0..=255u8).cycle().take(4096).collect()
}

pub fn flac_bytes() -> Vec<u8> {
    flac_with_metadata(&[])
}

fn flac_block(block_type: u8, last: bool, content: &[u8]) -> Vec<u8> {
    let mut block = vec![block_type | if last { 0x80 } else { 0 }];
    block.extend_from_slice(&(content.len() as u32).to_be_bytes()[1..]);
    block.extend_from_slice(content);
    block
}

/// A Vorbis comment block holding ARTIST=Keep.
pub fn flac_comment_block(last: bool) -> Vec<u8> {
    let item = b"ARTIST=Keep";
    let mut content = Vec::new();
    content.extend_from_slice(&3u32.to_le_bytes());
    content.extend_from_slice(b"ven");
    content.extend_from_slice(&1u32.to_le_bytes());
    content.extend_from_slice(&(item.len() as u32).to_le_bytes());
    content.extend_from_slice(item);
    flac_block(4, last, &content)
}

pub fn flac_padding_block() -> Vec<u8> {
    flac_block(1, true, &[0; 100])
}

/// A valid STREAMINFO, the given blocks, then bytes standing in for the audio frames.
pub fn flac_with_metadata(blocks: &[Vec<u8>]) -> Vec<u8> {
    let mut streaminfo = Vec::new();
    streaminfo.extend_from_slice(&[0x10, 0x00, 0x10, 0x00, 0, 0, 0, 0, 0, 0]);
    let (rate, channels, bits, samples) = (44100u64, 1u64, 16u64, 44100u64);
    let packed = (rate << 44) | ((channels - 1) << 41) | ((bits - 1) << 36) | samples;
    streaminfo.extend_from_slice(&packed.to_be_bytes());
    streaminfo.extend_from_slice(&[0; 16]); // MD5

    let mut bytes = b"fLaC".to_vec();
    bytes.extend_from_slice(&flac_block(0, blocks.is_empty(), &streaminfo));
    for block in blocks {
        bytes.extend_from_slice(block);
    }
    bytes.extend_from_slice(&flac_audio());
    bytes
}
