use crate::audio::http_stream::HttpStreamReader;
use crate::audio::audio_decoder::AudioDecoder;
use crate::audio::audio_source::StreamingAudioSource;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::Write;

pub struct AudioPipeline {
    pub sink: Option<Sink>,
    pub _stream: Option<OutputStream>,
    pub stream_handle: Option<OutputStreamHandle>,
    pub http_reader: Option<HttpStreamReader>,
    pub decoder: Option<Arc<Mutex<AudioDecoder>>>,
    pub recording_file: Option<Arc<Mutex<File>>>,
}

impl AudioPipeline {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        
        Ok(Self {
            sink: None,
            _stream: Some(stream),
            stream_handle: Some(stream_handle),
            http_reader: None,
            decoder: None,
            recording_file: None,
        })
    }
    
    pub async fn start_stream(
        &mut self, 
        url: &str, 
        recording_path: Option<&str>,
        runtime: &bevy_tokio_tasks::TokioTasksRuntime
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.stop_stream().await;
        
        println!("Starting audio pipeline for: {}", url);
        
        // Create HTTP reader
        let http_reader = HttpStreamReader::new(url).await?;
        
        // Create decoder
        let decoder = Arc::new(Mutex::new(AudioDecoder::new()));
        
        // Create recording file if requested
        let recording_file = if let Some(path) = recording_path {
            // Create directory if it doesn't exist
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = File::create(path)?;
            Some(Arc::new(Mutex::new(file)))
        } else {
            None
        };
        
        // Create audio source from decoder's sample buffer
        let sample_buffer = if let Ok(decoder_guard) = decoder.lock() {
            decoder_guard.get_sample_buffer()
        } else {
            return Err("Failed to access decoder".into());
        };
        
        let audio_source = StreamingAudioSource::new(sample_buffer);
        
        // Create sink and start playback
        if let Some(handle) = &self.stream_handle {
            let sink = Sink::try_new(handle)?;
            sink.append(audio_source);
            sink.play();
            self.sink = Some(sink);
            println!("Audio playback started");
        }
        
        // Start background task to read HTTP data and decode
        let http_reader_clone = http_reader.clone();
        let decoder_clone = decoder.clone();
        let recording_file_clone = recording_file.clone();
        
        runtime.spawn_background_task(move |_ctx| async move {
            let mut total_bytes = 0;
            let mut chunk_count = 0;
            
            while http_reader_clone.is_active().await {
                if let Some(chunk_result) = http_reader_clone.read_chunk().await {
                    match chunk_result {
                        Ok(chunk) => {
                            total_bytes += chunk.len();
                            chunk_count += 1;
                            
                            // Save to recording file if enabled
                            if let Some(ref file_mutex) = recording_file_clone {
                                if let Ok(mut file) = file_mutex.lock() {
                                    let _ = file.write_all(&chunk);
                                }
                            }
                            
                            // Feed data to decoder
                            if let Ok(mut decoder_guard) = decoder_clone.lock() {
                                if let Err(e) = decoder_guard.feed_data(chunk) {
                                    eprintln!("Decode error: {}", e);
                                }
                            }
                            
                            // Log progress every 50 chunks
                            if chunk_count % 50 == 0 {
                                println!("Processed {} chunks, {} KB total", chunk_count, total_bytes / 1024);
                            }
                        }
                        Err(e) => {
                            eprintln!("HTTP stream error: {}", e);
                            break;
                        }
                    }
                } else {
                    println!("HTTP stream ended");
                    break;
                }
            }
            
            println!("Audio pipeline background task ended. Processed {} chunks, {} KB total", 
                     chunk_count, total_bytes / 1024);
        });
        
        // Store references
        self.http_reader = Some(http_reader);
        self.decoder = Some(decoder);
        self.recording_file = recording_file;
        
        Ok(())
    }
    
    pub async fn stop_stream(&mut self) {
        println!("Stopping audio pipeline");
        
        // Stop HTTP reader
        if let Some(reader) = &self.http_reader {
            reader.stop().await;
        }
        
        // Stop audio playback
        if let Some(sink) = &self.sink {
            sink.stop();
        }
        
        // Reset decoder
        if let Some(decoder) = &self.decoder {
            if let Ok(mut decoder_guard) = decoder.lock() {
                decoder_guard.reset();
            }
        }
        
        // Close recording file
        if let Some(file_mutex) = &self.recording_file {
            if let Ok(mut file) = file_mutex.lock() {
                let _ = file.flush();
            }
        }
        
        // Clear references
        self.sink = None;
        self.http_reader = None;
        self.decoder = None;
        self.recording_file = None;
    }
    
    pub fn is_playing(&self) -> bool {
        self.sink.as_ref().map_or(false, |sink| !sink.empty())
    }
}