use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;
use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};
use std::io::Cursor;
use bytes::Bytes;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

pub struct AudioDecoder {
    buffer: Vec<u8>,
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    initialized: bool,
}

impl AudioDecoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            sample_buffer: Arc::new(Mutex::new(VecDeque::new())),
            initialized: false,
        }
    }
    
    pub fn get_sample_buffer(&self) -> Arc<Mutex<VecDeque<f32>>> {
        self.sample_buffer.clone()
    }
    
    pub fn feed_data(&mut self, data: Bytes) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Add new data to buffer
        self.buffer.extend_from_slice(&data);
        
        // Try to decode some audio
        self.try_decode()?;
        
        Ok(())
    }
    
    fn try_decode(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Need enough data to start decoding
        if self.buffer.len() < 16384 {
            return Ok(());
        }
        
        // Take a chunk to decode
        let chunk_size = std::cmp::min(self.buffer.len(), 65536);
        let decode_data = self.buffer.drain(0..chunk_size).collect::<Vec<u8>>();
        
        // Create a hint for AAC format
        let mut hint = Hint::new();
        hint.with_extension("aac");
        
        // Create a media source from our data
        let cursor = Cursor::new(decode_data);
        let media_source = MediaSourceStream::new(
            Box::new(ReadOnlySource::new(cursor)), 
            Default::default()
        );
        
        // Try to probe and decode
        match get_probe().format(&hint, media_source, &FormatOptions::default(), &MetadataOptions::default()) {
            Ok(probe_result) => {
                let mut format_reader = probe_result.format;
                
                // Find the first audio track
                if let Some(track) = format_reader
                    .tracks()
                    .iter()
                    .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL) {
                    
                    // Create a decoder for the track
                    match symphonia::default::get_codecs().make(&track.codec_params, &Default::default()) {
                        Ok(mut decoder) => {
                            // Try to decode packets
                            while let Ok(packet) = format_reader.next_packet() {
                                if packet.track_id() == track.id {
                                    match decoder.decode(&packet) {
                                        Ok(audio_buf) => {
                                            // Convert to f32 samples
                                            if let Some(samples) = self.convert_audio_buffer(&audio_buf) {
                                                // Add to sample buffer
                                                if let Ok(mut sample_buf) = self.sample_buffer.lock() {
                                                    sample_buf.extend(samples);
                                                    
                                                    // Keep buffer size reasonable (5 seconds of audio)
                                                    while sample_buf.len() > 44100 * 5 * 2 {
                                                        sample_buf.pop_front();
                                                    }
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            // Skip decode errors
                                            continue;
                                        }
                                    }
                                }
                            }
                            
                            if !self.initialized {
                                println!("Audio decoder initialized: {:?}", track.codec_params);
                                self.initialized = true;
                            }
                        }
                        Err(_) => {
                            // Can't create decoder
                        }
                    }
                }
            }
            Err(_) => {
                // Can't probe format - might need more data
            }
        }
        
        Ok(())
    }
    
    fn convert_audio_buffer(&self, audio_buf: &AudioBufferRef<f32>) -> Option<Vec<f32>> {
        let spec = *audio_buf.spec();
        let mut samples = Vec::new();
        
        // Handle different channel configurations
        match spec.channels.count() {
            1 => {
                // Mono - duplicate to stereo
                let channel = audio_buf.chan(0);
                for &sample in channel {
                    samples.push(sample); // Left
                    samples.push(sample); // Right (duplicate)
                }
            }
            2 => {
                // Stereo - interleave channels
                let left = audio_buf.chan(0);
                let right = audio_buf.chan(1);
                for (l, r) in left.iter().zip(right.iter()) {
                    samples.push(*l);
                    samples.push(*r);
                }
            }
            _ => {
                // Multi-channel - just take first two channels as stereo
                let left = audio_buf.chan(0);
                let right = audio_buf.chan(1);
                for (l, r) in left.iter().zip(right.iter()) {
                    samples.push(*l);
                    samples.push(*r);
                }
            }
        }
        
        if samples.is_empty() {
            None
        } else {
            Some(samples)
        }
    }
    
    pub fn reset(&mut self) {
        self.buffer.clear();
        if let Ok(mut sample_buf) = self.sample_buffer.lock() {
            sample_buf.clear();
        }
        self.initialized = false;
    }
}