// Copyright 2025 The Drasi Authors.
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

use redis::{aio::MultiplexedConnection, AsyncCommands, RedisResult};
use drasi_comms_redis_reader::reader::{RedisStreamRecordData};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::Receiver;

#[derive(Clone, Debug)]
pub struct RedisWriterSettings {
    pub host: String,
    pub port: u16,
    pub stream_name: String,
}

impl RedisWriterSettings {
    pub fn new(host: String, port: u16, stream_name: String) -> Self {
        RedisWriterSettings {
            host,
            port,
            stream_name,
        }
    }
}

pub struct RedisWriter {
    settings: RedisWriterSettings,
}

impl RedisWriter {
    pub fn new(settings: RedisWriterSettings) -> Self {
        RedisWriter { settings }
    }

    pub async fn start(&self, mut rx: Receiver<RedisStreamRecordData>) -> anyhow::Result<()> {
        log::debug!("Initializing RedisWriter with settings {:?}", self.settings);

        let client = redis::Client::open(format!("redis://{}:{}/", self.settings.host, self.settings.port))?;
        let mut con = client.get_multiplexed_async_connection().await?;

        let stream_key = self.settings.stream_name.clone();

        while let Some(record) = rx.recv().await {
            if let Err(e) = write_to_stream(&mut con, &stream_key, &record).await {
                log::error!("Failed to write to Redis stream: {:?}", e);
                // Optional: Implement retry logic or break the loop based on your requirements
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        Ok(())
    }
}

async fn write_to_stream(
    con: &mut MultiplexedConnection,
    stream_key: &str,
    record: &RedisStreamRecordData,
) -> anyhow::Result<String> {
    let serialized_data = serde_json::to_string(record)?;
    
    // Use XADD to append to the stream with automatic ID generation (*)
    let result: RedisResult<String> = con
        .xadd(stream_key, "*", &[("data", &serialized_data)])
        .await;

    match result {
        Ok(stream_id) => {
            log::debug!("Successfully wrote to stream {} with ID {}", stream_key, stream_id);
            Ok(stream_id)
        }
        Err(e) => Err(anyhow::anyhow!("Redis XADD error: {:?}", e)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisStreamReadResult {
    pub id: String,
    pub enqueue_time_ns: u64,
    pub dequeue_time_ns: u64,
    pub data: RedisStreamRecordData,
}
