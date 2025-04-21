// Copyright 2024 The Drasi Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use async_trait::async_trait;
use drasi_comms_abstractions::comms::{Headers, Invoker, Payload, Publisher};
use serde_json::Value;
use reqwest::Body;
use std::pin::Pin;
use futures::stream::{self, Stream, StreamExt};
pub struct DaprHttpPublisher {
    client: reqwest::Client,
    dapr_host: String,
    dapr_port: u16,
    pubsub: String,
    topic: String,
}

#[async_trait]
impl Publisher for DaprHttpPublisher {
    fn new(dapr_host: String, dapr_port: u16, pubsub: String, topic: String) -> Self {
        DaprHttpPublisher {
            client: reqwest::Client::new(),
            dapr_host,
            dapr_port,
            pubsub,
            topic,
        }
    }

    async fn publish(
        &self,
        data: Value,
        headers: Headers,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut request = self
            .client
            .post(format!(
                "http://{}:{}/v1.0/publish/{}/{}",
                self.dapr_host, self.dapr_port, self.pubsub, self.topic
            ))
            .json(&data);

        for (key, value) in headers.headers.iter() {
            request = request.header(key, value);
        }

        let response = request.send().await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    let error_message = format!(
                        "Dapr publish request failed with status: {} and body: {}",
                        resp.status(),
                        resp.text().await.unwrap_or_default()
                    );
                    Err(Box::from(error_message))
                }
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    
}

impl DaprHttpPublisher {
    pub async fn publish_bulk(
        &self,
        data: Value,
        headers: Headers,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut request = self
            .client
            .post(format!(
                "http://{}:{}/v1.0-alpha/publish/bulk/{}/{}",
                self.dapr_host, self.dapr_port, self.pubsub, self.topic
            ))
            .json(&data);

        for (key, value) in headers.headers.iter() {
            request = request.header(key, value);
        }

        let response = request.send().await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    let error_message = format!(
                        "Dapr publish request failed with status: {} and body: {}",
                        resp.status(),
                        resp.text().await.unwrap_or_default()
                    );
                    Err(Box::from(error_message))
                }
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DaprHttpInvoker {
    client: reqwest::Client,
    dapr_host: String,
    dapr_port: u16,
}

impl DaprHttpInvoker {
    pub fn new(dapr_host: String, dapr_port: u16) -> Self {
        DaprHttpInvoker {
            client: reqwest::Client::new(),
            dapr_host,
            dapr_port,
        }
    }
}

#[async_trait]
impl Invoker for DaprHttpInvoker {
    async fn invoke(
        &self,
        data: Payload<'_>,
        app_id: &str,
        method: &str,
        headers: Option<Headers>,
    ) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
        let url = format!(
            "http://{}:{}/v1.0/invoke/{}/method/{}",
            self.dapr_host, self.dapr_port, app_id, method
        );

        let request_headers = {
            let mut request_headers = reqwest::header::HeaderMap::new();
            if let Some(headers) = headers {
                for (key, value) in headers.headers.iter() {
                    request_headers
                        .insert(key.parse::<reqwest::header::HeaderName>()?, value.parse()?);
                }
            }

            if !request_headers.contains_key("Content-Type") {
                match data {
                    Payload::Json(_) => {
                        request_headers.insert("Content-Type", "application/json".parse()?);
                    }
                    Payload::JsonRef(_) => {
                        request_headers.insert("Content-Type", "application/json".parse()?);
                    }
                    Payload::Bytes(_) => {
                        request_headers.insert("Content-Type", "application/octet-stream".parse()?);
                    }
                    _ => {}
                }
            }

            request_headers
        };

        let request = self.client.post(url).headers(request_headers);

        let request = match data {
            Payload::Json(data) => request.json(&data),
            Payload::JsonRef(data) => request.json(data),
            Payload::Bytes(data) => request.body(data),
            _ => request,
        };

        let response = request.send().await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(resp.bytes().await?)
                } else {
                    let error_message = format!(
                        "Service invocation request failed with status: {} and body: {}",
                        resp.status(),
                        resp.text().await.unwrap_or_default()
                    );
                    Err(Box::from(error_message))
                }
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}


impl DaprHttpInvoker {
    pub async fn invoke_stream(
        &self,
        data: Payload<'_>,
        app_id: &str,
        method: &str,
        headers: Option<Headers>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>, Box<dyn std::error::Error>> {
        let url = format!(
            "http://{}:{}/v1.0/invoke/{}/method/{}",
            self.dapr_host, self.dapr_port, app_id, method
        );

        let mut request_headers = reqwest::header::HeaderMap::new();
        if let Some(headers) = headers {
            for (key, value) in headers.headers.iter() {
                request_headers
                    .insert(key.parse::<reqwest::header::HeaderName>()?, value.parse()?);
            }
        }

        if !request_headers.contains_key("Content-Type") {
            match data {
                Payload::Json(_) | Payload::JsonRef(_) => {
                    request_headers.insert("Content-Type", "application/json".parse()?);
                }
                Payload::Bytes(_) => {
                    request_headers.insert("Content-Type", "application/octet-stream".parse()?);
                }
                _ => {}
            }
        }

        // Add streaming-specific headers
        request_headers.insert("Transfer-Encoding", "chunked".parse()?);
        request_headers.insert("Accept", "application/octet-stream".parse()?);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120)) // Longer timeout for streaming
            .build()?;
            
        let request = client.post(url).headers(request_headers);

        // Create a proper streaming body based on payload type
        let request = match data {
            Payload::Json(data) => {
                // For large JSON, we could implement chunked serialization
                // For this example, we'll still serialize it once but prepare for proper streaming
                let json_bytes = serde_json::to_vec(&data)?;
                
                // Create chunks of reasonable size (e.g., 64KB)
                const CHUNK_SIZE: usize = 65536;
                let chunks = json_bytes
                    .chunks(CHUNK_SIZE)
                    .map(|chunk| Ok::<_, std::io::Error>(bytes::Bytes::copy_from_slice(chunk)))
                    .collect::<Vec<_>>();
                
                let stream = stream::iter(chunks);
                request.body(Body::wrap_stream(stream))
            },
            Payload::JsonRef(data) => {
                // Similar approach for JsonRef
                let json_bytes = serde_json::to_vec(data)?;
                
                const CHUNK_SIZE: usize = 65536;
                let chunks = json_bytes
                    .chunks(CHUNK_SIZE)
                    .map(|chunk| Ok::<_, std::io::Error>(bytes::Bytes::copy_from_slice(chunk)))
                    .collect::<Vec<_>>();
                
                let stream = stream::iter(chunks);
                request.body(Body::wrap_stream(stream))
            },
            Payload::Bytes(data) => {
                // Chunk the byte data
                const CHUNK_SIZE: usize = 65536;
                let chunks = data
                    .chunks(CHUNK_SIZE)
                    .map(|chunk| Ok::<_, std::io::Error>(bytes::Bytes::copy_from_slice(chunk)))
                    .collect::<Vec<_>>();
                
                let stream = stream::iter(chunks);
                request.body(Body::wrap_stream(stream))
            },
            _ => request,
        };

        // Send the request with streaming enabled
        let response = request.send().await?;

        if response.status().is_success() {
            // Process the streaming response
            let stream = response.bytes_stream().map(|result| {
                result.map_err(|e| e)
            });
            
            Ok(Box::pin(stream))
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            Err(Box::from(format!(
                "Service invocation failed with status: {} - {}",
                status, error_text
            )))
        }
    }
    // pub async fn invoke_stream(
    //     &self,
    //     data: Payload<'_>,
    //     app_id: &str,
    //     method: &str,
    //     headers: Option<Headers>,
    // ) -> Result<Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>, Box<dyn std::error::Error>> {
    //     let url = format!(
    //         "http://{}:{}/v1.0/invoke/{}/method/{}",
    //         self.dapr_host, self.dapr_port, app_id, method
    //     );

    //     let mut request_headers = reqwest::header::HeaderMap::new();
    //     if let Some(headers) = headers {
    //         for (key, value) in headers.headers.iter() {
    //             request_headers
    //                 .insert(key.parse::<reqwest::header::HeaderName>()?, value.parse()?);
    //         }
    //     }

    //     if !request_headers.contains_key("Content-Type") {
    //         match data {
    //             Payload::Json(_) | Payload::JsonRef(_) => {
    //                 request_headers.insert("Content-Type", "application/json".parse()?);
    //             }
    //             Payload::Bytes(_) => {
    //                 request_headers.insert("Content-Type", "application/octet-stream".parse()?);
    //             }
    //             _ => {}
    //         }
    //     }

    //     // Add chunked transfer encoding for streaming
    //     request_headers.insert("Transfer-Encoding", "chunked".parse()?);
        
    //     let request = self.client.post(url).headers(request_headers);

    //     let request = match data {
    //         Payload::Json(data) => {
    //             let size = serde_json::to_vec(&data).map(|v| v.len()).unwrap_or(0);
    //             println!("Payload size: {} bytes", size);
    //             let stream = stream::once(async move {
    //                 serde_json::to_vec(&data)
    //                     .map(bytes::Bytes::from)
    //                     .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    //             });
    //             request.body(Body::wrap_stream(stream))
    //         },
    //         Payload::Bytes(data) => {
    //             let stream = stream::once(async move { Ok::<_, std::io::Error>(bytes::Bytes::from(data)) });
    //             request.body(Body::wrap_stream(stream))
    //         }
    //         _ => request,
    //     };

    //     let response = request.send().await?;

    //     if response.status().is_success() {
    //         Ok(Box::pin(response.bytes_stream()))
    //     } else {
    //         Err(Box::from(format!(
    //             "Service invocation failed with status: {}",
    //             response.status()
    //         )))
    //     }
    // }
}