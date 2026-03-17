use base64::{engine::general_purpose, Engine as _};

use crate::provider::common::decode_header_value;
use crate::storage::memory;

pub(super) fn quote_imap_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

pub(super) fn slug(name: &str) -> String {
    name.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-")
        .trim_matches('-')
        .to_string()
}

pub(super) fn decode_imap_utf7(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'&' {
            result.push(bytes[i] as char);
            i += 1;
            continue;
        }

        i += 1;
        if i >= bytes.len() {
            result.push('&');
            break;
        }

        if bytes[i] == b'-' {
            result.push('&');
            i += 1;
            continue;
        }

        let start = i;
        while i < bytes.len() && bytes[i] != b'-' {
            i += 1;
        }

        if i >= bytes.len() {
            result.push('&');
            result.push_str(&String::from_utf8_lossy(&bytes[start..]));
            break;
        }

        let encoded = &bytes[start..i];
        i += 1;

        let decoded_utf16 = decode_modified_base64(encoded);
        if let Ok(s) = String::from_utf16(&decoded_utf16) {
            result.push_str(&s);
        } else {
            result.push('&');
            result.push_str(&String::from_utf8_lossy(encoded));
            result.push('-');
        }
    }

    result
}

pub(super) fn encode_imap_utf7(input: &str) -> String {
    let mut result = String::new();
    let mut non_ascii = String::new();

    let flush_non_ascii = |result: &mut String, non_ascii: &mut String| {
        if non_ascii.is_empty() {
            return;
        }

        let utf16 = non_ascii.encode_utf16().collect::<Vec<_>>();
        let mut bytes = Vec::with_capacity(utf16.len() * 2);
        for code_unit in utf16 {
            bytes.extend_from_slice(&code_unit.to_be_bytes());
        }

        let encoded = general_purpose::STANDARD.encode(bytes);
        let encoded = encoded.trim_end_matches('=').replace('/', ",");
        result.push('&');
        result.push_str(&encoded);
        result.push('-');
        non_ascii.clear();
    };

    for character in input.chars() {
        if character == '&' {
            flush_non_ascii(&mut result, &mut non_ascii);
            result.push_str("&-");
        } else if character.is_ascii() && matches!(character, ' '..='~') {
            flush_non_ascii(&mut result, &mut non_ascii);
            result.push(character);
        } else {
            non_ascii.push(character);
        }
    }

    flush_non_ascii(&mut result, &mut non_ascii);
    result
}

fn decode_modified_base64(input: &[u8]) -> Vec<u16> {
    let mut input_str = String::from_utf8_lossy(input).to_string();
    input_str = input_str.replace(',', "/");

    while !input_str.len().is_multiple_of(4) {
        input_str.push('=');
    }

    let decoded_bytes = match general_purpose::STANDARD.decode(&input_str) {
        Ok(bytes) => bytes,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    for chunk in decoded_bytes.chunks(2) {
        if chunk.len() == 2 {
            out.push(u16::from_be_bytes([chunk[0], chunk[1]]));
        }
    }

    out
}

pub(super) fn extract_date_from_body(raw: &[u8], _uid: u32) -> String {
    match mailparse::parse_mail(raw) {
        Ok(mail) => {
            let date_header = mail
                .headers
                .iter()
                .find(|h| h.get_key().eq_ignore_ascii_case("Date"))
                .map(|h| h.get_value());

            if let Some(date_str) = date_header {
                if let Ok(ts) = mailparse::dateparse(&date_str) {
                    let dt =
                        chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(chrono::Utc::now);
                    return dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                }
            }

            let received_header = mail
                .headers
                .iter()
                .find(|h| h.get_key().eq_ignore_ascii_case("Received"))
                .map(|h| h.get_value());

            if let Some(received_str) = received_header {
                if let Some(date_part) = received_str.split(';').next_back() {
                    let date_part = date_part.trim();
                    if let Ok(ts) = mailparse::dateparse(date_part) {
                        let dt = chrono::DateTime::from_timestamp(ts, 0)
                            .unwrap_or_else(chrono::Utc::now);
                        return dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                    }
                }
            }

            memory::current_timestamp()
        }
        Err(_) => memory::current_timestamp(),
    }
}

pub(super) fn parse_envelope(
    env: &imap_proto::types::Envelope,
) -> (String, String, String, Vec<String>, Vec<String>, String) {
    let subject = env
        .subject
        .as_ref()
        .map(|s| decode_header_value(s))
        .unwrap_or_else(|| "(No subject)".into());

    let (from, from_email) = env
        .from
        .as_ref()
        .and_then(|addrs| addrs.first())
        .map(|addr: &imap_proto::types::Address| {
            let name = addr
                .name
                .as_ref()
                .map(|n| decode_header_value(n))
                .unwrap_or_default();
            let email = envelope_address(addr);
            let display = if name.is_empty() { email.clone() } else { name };
            (display, email)
        })
        .unwrap_or_else(|| ("Unknown".into(), "unknown@unknown".into()));

    let to = env
        .to
        .as_ref()
        .map(|addrs| addrs.iter().map(envelope_address).collect())
        .unwrap_or_default();
    let cc = env
        .cc
        .as_ref()
        .map(|addrs| addrs.iter().map(envelope_address).collect())
        .unwrap_or_default();

    let date = env
        .date
        .as_ref()
        .map(|d| {
            let raw = String::from_utf8_lossy(d).to_string();
            mailparse::dateparse(&raw)
                .map(|ts| {
                    let dt =
                        chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(chrono::Utc::now);
                    dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                })
                .unwrap_or_else(|e| {
                    eprintln!("[imap] WARNING: failed to parse date '{}': {:?}", raw, e);
                    String::new()
                })
        })
        .unwrap_or_default();

    (subject, from, from_email, to, cc, date)
}

fn envelope_address(addr: &imap_proto::types::Address) -> String {
    let mailbox = addr
        .mailbox
        .as_ref()
        .map(|m| String::from_utf8_lossy(m).to_string())
        .unwrap_or_default();
    let host = addr
        .host
        .as_ref()
        .map(|h| String::from_utf8_lossy(h).to_string())
        .unwrap_or_default();
    format!("{mailbox}@{host}")
}
