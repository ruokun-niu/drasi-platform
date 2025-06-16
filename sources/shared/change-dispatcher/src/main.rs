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

use change_dispatcher_config::ChangeDispatcherConfig;
use drasi_comms_abstractions::comms::Payload;
use log::info;
use serde_json::Value;

use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures::future::join_all;
use tokio::sync::Semaphore;
use tokio::signal::unix::{signal, SignalKind};

use drasi_comms_abstractions::comms::{Headers, Invoker};
use drasi_comms_dapr::comms::DaprHttpInvoker;
use drasi_comms_redis_reader::reader::{RedisReader, RedisReaderSettings, RedisStreamReadResult, RedisStreamRecordData};

use tokio::sync::mpsc::Receiver;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

mod change_dispatcher_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    info!("Starting Source Change Dispatcher");

    let config = ChangeDispatcherConfig::new();
    info!("Initializing SourceID: {} ...", config.source_id);

    let dapr_http_port = match config.dapr_http_port.parse::<u16>() {
        Ok(port) => port,
        Err(_) => {
            panic!("Invalid DAPR_HTTP_PORT: {}", config.dapr_http_port);
        }
    };

    let invoker = DaprHttpInvoker::new("127.0.0.1".to_string(), dapr_http_port);
    let shared_state = Arc::new(AppState {
        config: config.clone(),
        invoker,
    });

    let redis_settings = RedisReaderSettings::new(
        "drasi-redis".to_string(), // Replace with config.redis_host if available
        6379,                 
        format!("{}-dispatch", config.source_id), // Match Dapr topic
        Some(true),
    );

    let reader = RedisReader::new(redis_settings);
    let rx = reader.start().await?;

    // Spawn task to process messages from RedisReader
    let state_clone = shared_state.clone();
    tokio::spawn(async move {
        process_messages(rx, state_clone).await;
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .with_state(shared_state.clone());
    let addr = "0.0.0.0:3000".to_string(); // Dapr expects port 3000
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    log::info!("Server started at: {}", addr);
    // Run server and handle SIGTERM concurrently
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        result = axum::serve(listener, app) => {
            if let Err(e) = result {
                log::error!("Server error: {:?}", e);
                return Err(e.into());
            }
        }
        _ = sigterm.recv() => {
            log::info!("Received SIGTERM. Exiting...");
        }
    }

    
    Ok(())
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}


async fn process_messages(mut rx: Receiver<RedisStreamReadResult>, state: Arc<AppState>) {
    while let Some(message) = rx.recv().await {
        // Capture receive time in nanoseconds
        let receive_time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
        // Extract traceparent (assuming Dapr includes it in the data payload)
        let traceparent = match message.data.traceparent {
            Some(ref tp) => tp.to_string(),
            None => "".to_string(),
        };
        // log::info!(
        //     "Received message: {}",
        //     serde_json::to_string_pretty(&message).unwrap_or_default()
        // );

        // Process the message using existing process_changes function
        match process_changes(
            &state.invoker,
            &message.data.data, // Pass the data field
            &state.config,
            traceparent,
            receive_time,
        ).await {
            Ok(_) => {},
            Err(e) => log::error!("Error processing message: {:?}", e),
        }
    }
}

struct AppState {
    config: ChangeDispatcherConfig,
    invoker: DaprHttpInvoker,
}


async fn process_changes(
    invoker: &DaprHttpInvoker,
    changes: &Value,
    _config: &ChangeDispatcherConfig,
    traceparent: String,
    receive_time: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut start_time = receive_time;

    let changes = changes
        .as_array()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Changes must be an array"))?;

    for (index, change_event) in changes.iter().enumerate() {
        // For the first change, we will use the receive_time from the pubsub
        // For the rest of the changes, we will use the time when the change event is processed
        if index > 0 {
            start_time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
        }
        let mut dispatch_event = change_event.clone();
        dispatch_event["metadata"]["tracking"]["source"]["changeDispatcherStart_ns"] =
            start_time.into();

        let subscriptions = change_event["subscriptions"].as_array().ok_or_else(|| {
            Box::<dyn std::error::Error>::from("Error getting subscriptions from change event")
        })?;

        let query_nodes: HashSet<&str> = subscriptions
            .iter()
            .map(|x| x["queryNodeId"].as_str().unwrap_or_default())
            .collect();

        for query_node_id in query_nodes {
            let app_id = format!("{}-publish-api", query_node_id);

            let queries: Vec<_> = subscriptions
                .iter()
                .filter(|x| x["queryNodeId"] == query_node_id)
                .map(|x| &x["queryId"])
                .collect();
            dispatch_event["queries"] = match serde_json::to_value(queries) {
                Ok(val) => val,
                Err(_) => {
                    return Err(Box::<dyn std::error::Error>::from(
                        "Error serializing queries into json value",
                    ));
                }
            };

            let mut headers = HashMap::new();
            if !traceparent.is_empty() {
                headers.insert("traceparent".to_string(), traceparent.clone());
            }
            let headers = Headers::new(headers);

            // End time, measured in nanoseconds
            dispatch_event["metadata"]["tracking"]["source"]["changeDispatcherEnd_ns"] =
                match serde_json::to_value(
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
                ) {
                    Ok(val) => val,
                    Err(_) => {
                        return Err(Box::<dyn std::error::Error>::from(
                            "Error serializing timestamp into json value",
                        ));
                    }
                };

            let payload_size = match serde_json::to_string(&dispatch_event) {
                Ok(val) => val.len(),
                Err(_) => {
                    return Err(Box::<dyn std::error::Error>::from(
                        "Error serializing dispatch event into string",
                    ));
                }
            };
            info!(
                "Dispatching change - id:{}, app_id:{}, payload_size:{}",
                change_event["id"], app_id, payload_size
            );
            match invoker
                .invoke(
                    Payload::Json(dispatch_event.clone()),
                    &app_id,
                    "change",
                    Some(headers),
                )
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    return Err(Box::<dyn std::error::Error>::from(format!(
                        "Error invoking app: {}",
                        e
                    )));
                }
            }
        }
    }
    Ok(())
}
