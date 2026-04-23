pub fn mix_aligned_sources_to_mono(microphone: &[i16], system: &[i16]) -> Vec<i16> {
    let mixed_len = microphone.len().max(system.len());

    (0..mixed_len)
        .map(|index| {
            let mic = i32::from(*microphone.get(index).unwrap_or(&0));
            let sys = i32::from(*system.get(index).unwrap_or(&0));
            let average = (mic + sys) / 2;
            average.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
        })
        .collect()
}
