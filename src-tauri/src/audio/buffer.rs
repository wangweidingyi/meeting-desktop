#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmSampleBuffer {
    max_samples: usize,
    samples: Vec<i16>,
}

impl PcmSampleBuffer {
    pub fn new(max_samples: usize) -> Self {
        Self {
            max_samples,
            samples: Vec::new(),
        }
    }

    pub fn push(&mut self, incoming: &[i16]) -> usize {
        self.samples.extend_from_slice(incoming);

        if self.samples.len() <= self.max_samples {
            return 0;
        }

        let overflow = self.samples.len() - self.max_samples;
        self.samples.drain(0..overflow);
        overflow
    }

    pub fn take(&mut self, count: usize) -> Vec<i16> {
        let take_count = count.min(self.samples.len());
        self.samples.drain(0..take_count).collect()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}
