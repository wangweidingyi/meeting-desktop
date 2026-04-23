pub fn duration_ms_for_samples(sample_count: usize, sample_rate_hz: u32, channels: u16) -> u32 {
    if sample_count == 0 || sample_rate_hz == 0 || channels == 0 {
        return 0;
    }

    let frames = sample_count as u64 / u64::from(channels);
    ((frames * 1000) / u64::from(sample_rate_hz)) as u32
}

pub fn align_stream_start_ms(microphone_start_ms: u64, system_start_ms: u64) -> u64 {
    microphone_start_ms.max(system_start_ms)
}
