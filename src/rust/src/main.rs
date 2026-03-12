use std::io::{self, BufRead, Write};

mod core;
mod models;
mod protocol;
mod provider;
mod storage;

use core::context::app_context;
use protocol::{BackendRequestEnvelope, BackendResponse};

fn main() {
    let app_context = app_context();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };

        if line.trim().is_empty() {
            continue;
        }

        let request = match serde_json::from_str::<BackendRequestEnvelope>(&line) {
            Ok(request) => request,
            Err(error) => {
                let response = BackendResponse::error(
                    0,
                    protocol::BackendError::internal(format!("Invalid request: {error}")),
                );
                if write_response(&mut writer, &response).is_err() {
                    break;
                }
                continue;
            }
        };

        let response = match core::app::handle(&app_context, request.request) {
            Ok(result) => BackendResponse::success(request.id, result),
            Err(error) => BackendResponse::error(request.id, error),
        };

        if write_response(&mut writer, &response).is_err() {
            break;
        }
    }
}

fn write_response(writer: &mut impl Write, response: &BackendResponse) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, response)?;
    writer.write_all(b"\n")?;
    writer.flush()
}
