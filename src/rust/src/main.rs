use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::mpsc;

mod core;
mod models;
mod protocol;
mod provider;
mod storage;

use core::context::app_context;
use protocol::{BackendRequestEnvelope, BackendResponse};

#[tokio::main]
async fn main() {
    eprintln!("[backend] starting...");
    let start = Instant::now();
    let app_context = app_context();
    let provider = app_context.provider();
    eprintln!("[backend] initialized provider '{}' in {:?}", provider.backend_name(), start.elapsed());

    let stdin = BufReader::new(tokio::io::stdin());
    let (tx, mut rx) = mpsc::unbounded_channel::<BackendResponse>();

    // Dedicated writer task owns stdout — avoids Send issues with StdoutLock.
    let writer_handle = tokio::spawn(async move {
        let mut stdout = BufWriter::new(tokio::io::stdout());
        while let Some(response) = rx.recv().await {
            let Ok(json) = serde_json::to_vec(&response) else { continue };
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
                let _ = tx.send(response);
                continue;
            }
        };

        let request_id = request.id;
        let method = request.request.method_name();
        eprintln!("[backend] req #{request_id} {method}");

        let tx = tx.clone();
        tokio::spawn(async move {
            let start = Instant::now();

            // Nested spawn to catch panics (JoinError on panic)
            let inner = tokio::spawn(async move {
                core::app::handle_with_provider(provider, request.request).await
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

            if tx.send(response).is_err() {
                eprintln!("[backend] req #{request_id}: channel closed, response dropped");
            }
        });
    }

    // Drop the sender so the writer task exits when all in-flight requests finish.
    drop(tx);
    let _ = writer_handle.await;
    eprintln!("[backend] shutdown complete");
}
