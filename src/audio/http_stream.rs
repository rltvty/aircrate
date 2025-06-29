use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use bytes::Bytes;

#[derive(Clone)]
pub struct HttpStreamReader {
    pub receiver: Arc<Mutex<mpsc::Receiver<Result<Bytes, reqwest::Error>>>>,
    pub is_active: Arc<Mutex<bool>>,
}

impl HttpStreamReader {
    pub async fn new(url: &str) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::new();
        
        // Make the request with streaming enabled
        let response = client
            .get(url)
            .header("User-Agent", "AirCrate/1.0")
            .header("Accept", "audio/aac, audio/mpeg, */*")
            .send()
            .await?;
        
        println!("HTTP Response Status: {}", response.status());
        println!("Content-Type: {:?}", response.headers().get("content-type"));
        
        let (tx, rx) = mpsc::channel::<Result<Bytes, reqwest::Error>>(100);
        let is_active = Arc::new(Mutex::new(true));
        let is_active_clone = is_active.clone();
        
        // Spawn a task to read the stream
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            
            while *is_active_clone.lock().await {
                match stream.try_next().await {
                    Ok(Some(chunk)) => {
                        if tx.send(Ok(chunk)).await.is_err() {
                            // Receiver dropped, stop streaming
                            break;
                        }
                    }
                    Ok(None) => {
                        // Stream ended
                        println!("HTTP stream ended");
                        break;
                    }
                    Err(e) => {
                        println!("HTTP stream error: {}", e);
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }
            
            *is_active_clone.lock().await = false;
            println!("HTTP stream reader task ended");
        });
        
        Ok(HttpStreamReader {
            receiver: Arc::new(Mutex::new(rx)),
            is_active,
        })
    }
    
    pub async fn read_chunk(&self) -> Option<Result<Bytes, reqwest::Error>> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }
    
    pub async fn stop(&self) {
        *self.is_active.lock().await = false;
    }
    
    pub async fn is_active(&self) -> bool {
        *self.is_active.lock().await
    }
}

// Add the missing import
use futures_util::TryStreamExt;