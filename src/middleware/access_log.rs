use chrono::{DateTime, Utc};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::error;

#[derive(Debug, Clone, Serialize)]
pub struct AccessLogEntry {
    pub timestamp: DateTime<Utc>,
    pub client_ip: String,
    pub method: String,
    pub uri: String,
    pub protocol: String,
    pub status: u16,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub duration_ms: f64,
    pub backend: Option<String>,
    pub request_id: String,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
}

impl AccessLogEntry {
    pub fn new(
        client_addr: SocketAddr,
        method: &str,
        uri: &str,
        request_id: String,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            client_ip: client_addr.ip().to_string(),
            method: method.to_string(),
            uri: uri.to_string(),
            protocol: "HTTP/1.1".to_string(),
            status: 0,
            bytes_sent: 0,
            bytes_received: 0,
            duration_ms: 0.0,
            backend: None,
            request_id,
            user_agent: None,
            referer: None,
        }
    }

    pub fn complete(
        mut self,
        status: u16,
        bytes_sent: u64,
        duration: Duration,
        backend: Option<String>,
    ) -> Self {
        self.status = status;
        self.bytes_sent = bytes_sent;
        self.duration_ms = duration.as_secs_f64() * 1000.0;
        self.backend = backend;
        self
    }

    pub fn to_combined_format(&self) -> String {
        format!(
            "{} - - [{}] \"{} {} {}\" {} {} \"{}\" \"{}\" {}ms",
            self.client_ip,
            self.timestamp.format("%d/%b/%Y:%H:%M:%S %z"),
            self.method,
            self.uri,
            self.protocol,
            self.status,
            self.bytes_sent,
            self.referer.as_deref().unwrap_or("-"),
            self.user_agent.as_deref().unwrap_or("-"),
            self.duration_ms as u64,
        )
    }

    // pub fn to_csv(&self) -> String {
    //     format!(
    //         "{},{},{},{},{},{},{},{},{},{},{},{},{}",
    //         self.timestamp.format("%d/%b/%Y:%H:%M:%S %z"),
    //         self.client_ip,
    //         self.method,
    //         self.uri,
    //         self.protocol,
    //         self.status,
    //         self.bytes_sent,
    //         self.bytes_received,
    //         self.duration_ms as u64,
    //         self.backend.as_deref().unwrap_or("-"),
    //         self.request_id,
    //         self.user_agent.as_deref().unwrap_or("-"),
    //         self.referer.as_deref().unwrap_or("-"),
    //     )
    // }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Clone, Copy)]
pub enum LogFormat {
    Combined,
    Json,
}

pub struct AccessLogger {
    sender: mpsc::UnboundedSender<AccessLogEntry>,
    format: LogFormat,
}

impl AccessLogger {
    pub fn new(output_path: Option<String>, format: LogFormat) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<AccessLogEntry>();

        tokio::spawn(async move {
            let mut file = if let Some(path) = output_path {
                Some(
                    tokio::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&path)
                        .await
                        .ok(),
                )
            } else {
                None
            };

            while let Some(entry) = rx.recv().await {
                let line = match format {
                    LogFormat::Combined => entry.to_combined_format(),
                    LogFormat::Json => entry.to_json(),
                    // LogFormat::Csv => entry.to_csv(),
                };

                if let Some(Some(ref mut f)) = file {
                    if let Err(e) = f.write_all(format!("{}\n", line).as_bytes()).await {
                        error!("Failed to write access log: {}", e);
                    }
                } else {
                    println!("{}", line);
                }
            }
        });

        Self { sender: tx, format }
    }

    pub fn log(&self, entry: AccessLogEntry) {
        let _ = self.sender.send(entry);
    }
}

impl Clone for AccessLogger {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            format: match self.format {
                LogFormat::Combined => LogFormat::Combined,
                LogFormat::Json => LogFormat::Json,
            },
        }
    }
}
