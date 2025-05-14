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

use redis::{aio::MultiplexedConnection, streams::{StreamReadOptions, StreamReadReply}, AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Debug)]
pub struct RedisReaderSettings {
    pub host: String,
    pub port: u16,
    pub process_old_entries: bool,
    pub stream_name: String,
}

impl RedisReaderSettings {
    pub fn new(host: String, port: u16, stream_name: String, process_old_entries: Option<bool>) -> Self {
        RedisReaderSettings {
            host,
            port,
            process_old_entries: process_old_entries.unwrap_or(false),
            stream_name,
        }
    }
}

pub struct RedisReader {
    settings: RedisReaderSettings,
}

impl RedisReader {
    pub fn new(settings: RedisReaderSettings) -> Self {
        RedisReader { settings }
    }

    pub async fn start(&self) -> anyhow::Result<Receiver<RedisStreamReadResult>> {
        log::debug!("Initializing RedisReader with settings {:?}", self.settings);

        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        let settings = self.settings.clone();

        tokio::spawn(async move {
            if let Err(e) = reader_task(settings, tx).await {
                log::error!("Reader task failed: {:?}", e);
            }
        });

        Ok(rx)
    }
}

async fn reader_task(settings: RedisReaderSettings, tx: Sender<RedisStreamReadResult>) -> anyhow::Result<()> {
    log::debug!("Starting Redis reader task");

    let client = redis::Client::open(format!("redis://{}:{}/", settings.host, settings.port))?;
    let mut con = client.get_multiplexed_async_connection().await?;

    let stream_key = settings.stream_name;
    let mut last_id = match settings.process_old_entries {
        true => "0-0".to_string(),
        false =>  "$".to_string(),
    };


    let opts = StreamReadOptions::default().count(1).block(5000); // Reduced block time to 100ms

    loop {
        match read_stream(&mut con, &stream_key, &last_id, &opts).await {
            Ok(messages) => {
                for message in messages {
                    last_id = message.id.clone();
                    if let Err(e) = tx.send(message).await {
                        log::error!("Error sending message to channel: {:?}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Error reading from Redis stream: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

async fn read_stream(
    con: &mut MultiplexedConnection,
    stream_key: &str,
    last_id: &str,
    opts: &StreamReadOptions,
) -> anyhow::Result<Vec<RedisStreamReadResult>> {
    let xread_result: RedisResult<StreamReadReply> = con.xread_options(&[stream_key], &[last_id], opts).await;
    let xread_result = xread_result?;

    let dequeue_time_ns = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_nanos() as u64;

    let mut messages = Vec::new();

    for key in xread_result.keys {
        for id in key.ids {
            let stream_id = id.id.to_string();
            let enqueue_time_ns = stream_id
                .split('-')
                .next()
                .ok_or_else(|| anyhow::anyhow!("Invalid stream ID format"))?
                .parse::<u64>()?
                * 1_000_000;

            let data = id.map.get("data").ok_or_else(|| anyhow::anyhow!("No data field in stream entry"))?;

            match data {
                redis::Value::BulkString(bs_data) => {
                    match String::from_utf8(bs_data.to_vec()) {
                        Ok(data_str) => {
                            match serde_json::from_str::<RedisStreamRecordData>(&data_str) {
                                Ok(record) => {
                                    messages.push(RedisStreamReadResult {
                                        id: stream_id,
                                        enqueue_time_ns,
                                        dequeue_time_ns,
                                        data: record,
                                    });
                                }
                                Err(e) => {
                                    println!("data_str: {:?}",  serde_json::to_string_pretty(&data_str).unwrap_or_default());
                                    log::error!("Failed to deserialize message: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Invalid UTF-8 data: {:?}", e);
                        }
                    }
                },
                _ => {
                    log::error!("Data is not a BulkString");
                }
            }
        }
    }

    Ok(messages)
}


// copied from testframework
#[derive(Debug, Serialize, Deserialize)]
pub struct RedisStreamReadResult {
    pub id: String,
    pub enqueue_time_ns: u64,
    pub dequeue_time_ns: u64,
    pub data: RedisStreamRecordData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisStreamRecordData {
    pub data: serde_json::Value,
    pub id: String,
    pub traceparent: Option<String>,
    pub tracestate: Option<String>,
    pub topic: String,
}

