use rodio::Source;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Clone)]
pub struct StreamingAudioSource {
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    sample_rate: u32,
    channels: u16,
    position: usize,
}

impl StreamingAudioSource {
    pub fn new(sample_buffer: Arc<Mutex<VecDeque<f32>>>) -> Self {
        Self {
            sample_buffer,
            sample_rate: 44100, // Standard sample rate
            channels: 2,       // Stereo
            position: 0,
        }
    }
    
    pub fn has_samples(&self) -> bool {
        if let Ok(buffer) = self.sample_buffer.lock() {
            !buffer.is_empty()
        } else {
            false
        }
    }
}

impl Source for StreamingAudioSource {
    fn current_frame_len(&self) -> Option<usize> {
        None // Unknown frame length for streaming
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Infinite duration for streaming
    }
}

impl Iterator for StreamingAudioSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(mut buffer) = self.sample_buffer.lock() {
            if let Some(sample) = buffer.pop_front() {
                self.position += 1;
                Some(sample)
            } else {
                // No samples available, return silence
                Some(0.0)
            }
        } else {
            // Lock failed, return silence
            Some(0.0)
        }
    }
}