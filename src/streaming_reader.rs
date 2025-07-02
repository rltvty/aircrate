use std::collections::VecDeque;
use std::io::{Read, Result as IoResult, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

// Streaming reader that implements Read + Seek for use with Rodio's decoder
#[derive(Clone)]
pub struct StreamingReader {
    receiver: Arc<Mutex<std::sync::mpsc::Receiver<Vec<u8>>>>,
    buffer: VecDeque<u8>,
    position: u64,
    finished: bool,
}

impl StreamingReader {
    pub fn new(receiver: std::sync::mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            buffer: VecDeque::new(),
            position: 0,
            finished: false,
        }
    }

    fn fill_buffer(&mut self) -> IoResult<()> {
        if self.finished {
            return Ok(());
        }

        // Try to receive more data without blocking too long
        let result = {
            let receiver = self.receiver.lock().unwrap();
            receiver.recv_timeout(std::time::Duration::from_millis(100))
        };

        match result {
            Ok(chunk) => {
                self.buffer.extend(chunk);

                //println!("🔧 StreamingReader buffer: {} bytes", self.buffer.len());
                Ok(())
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // No data available right now, that's okay for streaming
                if self.buffer.len() < 1024 {
                    println!(
                        "⚠️  StreamingReader timeout with low buffer: {} bytes",
                        self.buffer.len()
                    );
                }
                Ok(())
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                println!("Stream disconnnected.");
                // Stream has ended
                self.finished = true;
                Ok(())
            }
        }
    }
}

impl Read for StreamingReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        // Keep trying to fill buffer until we have data or stream is finished
        while self.buffer.is_empty() && !self.finished {
            self.fill_buffer()?;
            if self.buffer.is_empty() && !self.finished {
                // Still no data, wait a bit longer
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        let bytes_to_read = std::cmp::min(buf.len(), self.buffer.len());

        if bytes_to_read == 0 {
            return Ok(0); // EOF - only when stream is actually finished
        }

        // Copy data from our buffer to the output buffer
        for i in 0..bytes_to_read {
            buf[i] = self.buffer.pop_front().unwrap();
        }

        self.position += bytes_to_read as u64;
        Ok(bytes_to_read)
    }
}

impl Seek for StreamingReader {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        match pos {
            SeekFrom::Current(0) => Ok(self.position),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Seeking not supported in streaming mode",
            )),
        }
    }
}

unsafe impl Send for StreamingReader {}
unsafe impl Sync for StreamingReader {}
