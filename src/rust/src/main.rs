use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::mpsc;

mod config;
mod core;
mod models;
mod oauth;
mod protocol;
mod provider;
mod realtime;
mod storage;

use core::context::app_context;
use protocol::{BackendMessage, BackendRequestEnvelope, BackendResponse};
use realtime::RealtimeController;

#[tokio::main]
async fn main() {
    eprintln!("[backend] starting...");
    let start = Instant::now();
    let app_context = app_context();
    let registry = *app_context.registry();
    eprintln!(
        "[backend] initialized provider '{}' in {:?}",
        "imap-smtp/pop3-smtp",
        start.elapsed()
    );

    let stdin = BufReader::new(tokio::io::stdin());
    let (tx, mut rx) = mpsc::unbounded_channel::<BackendMessage>();
    let realtime = RealtimeController::new();
    realtime.reconcile(tx.clone());

    // Dedicated writer task owns stdout — avoids Send issues with StdoutLock.
    let writer_handle = tokio::spawn(async move {
        let mut stdout = BufWriter::new(tokio::io::stdout());
        while let Some(message) = rx.recv().await {
            let Ok(json) = serde_json::to_vec(&message) else {
                continue;
            };
            if stdout.write_all(&json).await.is_err()
                || stdout.write_all(b"\n").await.is_err()
                || stdout.flush().await.is_err()
            {
                eprintln!("[backend] writer: stdout write failed, exiting");
                break;
            }
        }
        eprintln!("[backend] writer: channel closed, exiting");
    });

    eprintln!("[backend] ready, reading stdin");

    let mut lines = stdin.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let request = match serde_json::from_str::<BackendRequestEnvelope>(&line) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("[backend] parse error: {error}");
                eprintln!("[backend] raw input: {line}");
                let response = BackendResponse::error(
                    0,
                    protocol::BackendError::internal(format!("Invalid request: {error}")),
                );
                let _ = tx.send(BackendMessage::response(response));
                continue;
            }
        };

        let request_id = request.id;
        let method = request.request.method_name();
        eprintln!("[backend] req #{request_id} {method}");

        let tx = tx.clone();
        let realtime = realtime.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            let registry = registry;

            // Nested spawn to catch panics (JoinError on panic)
            let inner = tokio::spawn(async move {
                core::app::handle_request(&registry, request.request).await
            });

            let response = match inner.await {
                Ok(Ok(result)) => BackendResponse::success(request_id, result),
                Ok(Err(error)) => BackendResponse::error(request_id, error),
                Err(join_error) => {
                    let msg = if join_error.is_panic() {
                        let panic_value = join_error.into_panic();
                        panic_value
                            .downcast_ref::<&str>()
                            .map(|s| s.to_string())
                            .or_else(|| panic_value.downcast_ref::<String>().cloned())
                            .unwrap_or_else(|| "unknown panic".into())
                    } else {
                        "Task cancelled".into()
                    };
                    eprintln!("[backend] PANIC in req #{request_id} {method}: {msg}");
                    BackendResponse::error(
                        request_id,
                        protocol::BackendError::internal(format!("Internal panic: {msg}")),
                    )
                }
            };

            let elapsed = start.elapsed();
            let status = match &response {
                BackendResponse::Success(_) => "ok",
                BackendResponse::Error(_) => "error",
            };
            eprintln!("[backend] req #{request_id} {method} → {status} ({elapsed:.1?})");

            if tx.send(BackendMessage::response(response)).is_err() {
                eprintln!("[backend] req #{request_id}: channel closed, response dropped");
            }

            realtime.reconcile(tx.clone());
        });
    }

    // Drop the sender so the writer task exits when all in-flight requests finish.
    realtime.shutdown();
    drop(tx);
    let _ = writer_handle.await;
    eprintln!("[backend] shutdown complete");
}
