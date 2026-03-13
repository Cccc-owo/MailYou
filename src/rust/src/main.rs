use std::io::{self, BufRead, BufWriter, Write};
use std::panic::AssertUnwindSafe;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

mod core;
mod models;
mod protocol;
mod provider;
mod storage;

use core::context::app_context;
use protocol::{BackendRequestEnvelope, BackendResponse};

fn main() {
    eprintln!("[backend] starting...");
    let start = Instant::now();
    let app_context = app_context();
    let provider = app_context.provider();
    eprintln!("[backend] initialized provider '{}' in {:?}", provider.backend_name(), start.elapsed());

    let stdin = io::stdin();

    // Channel for worker threads to send responses back to the writer thread.
    let (tx, rx) = mpsc::channel::<BackendResponse>();

    // Dedicated writer thread owns stdout — avoids Send issues with StdoutLock.
    let writer_handle = thread::spawn(move || {
        let stdout = io::stdout();
        let mut writer = BufWriter::new(stdout.lock());
        for response in rx {
            if write_response(&mut writer, &response).is_err() {
                eprintln!("[backend] writer: stdout write failed, exiting");
                break;
            }
        }
        eprintln!("[backend] writer: channel closed, exiting");
    });

    eprintln!("[backend] ready, reading stdin");

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            eprintln!("[backend] stdin closed");
            break;
        };

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
        thread::spawn(move || {
            let start = Instant::now();

            // catch_unwind ensures a response is ALWAYS sent, even if the handler panics
            // (e.g. due to a poisoned Mutex).
            let response =
                std::panic::catch_unwind(AssertUnwindSafe(|| {
                    match core::app::handle_with_provider(provider, request.request) {
                        Ok(result) => BackendResponse::success(request_id, result),
                        Err(error) => BackendResponse::error(request_id, error),
                    }
                }))
                .unwrap_or_else(|panic_value| {
                    let msg = panic_value
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| panic_value.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic".into());
                    eprintln!("[backend] PANIC in req #{request_id} {method}: {msg}");
                    BackendResponse::error(
                        request_id,
                        protocol::BackendError::internal(format!("Internal panic: {msg}")),
                    )
                });

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

    // Drop the sender so the writer thread exits when all in-flight requests finish.
    drop(tx);
    let _ = writer_handle.join();
    eprintln!("[backend] shutdown complete");
}

fn write_response(writer: &mut impl Write, response: &BackendResponse) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, response)?;
    writer.write_all(b"\n")?;
    writer.flush()
}
