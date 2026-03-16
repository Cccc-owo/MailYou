use crate::models::{AccountSetupDraft, AttachmentMeta};
use crate::protocol::BackendError;

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

pub fn validate_draft(draft: &AccountSetupDraft) -> Result<(), BackendError> {
    if matches!(draft.auth_mode, crate::models::AccountAuthMode::Oauth) {
        if draft.incoming_protocol != "imap" {
            return Err(BackendError::validation("OAuth accounts currently require IMAP"));
        }

        if draft.email.trim().is_empty()
            || draft.username.trim().is_empty()
            || draft.incoming_host.trim().is_empty()
            || draft.outgoing_host.trim().is_empty()
        {
            return Err(BackendError::validation("All OAuth account fields are required"));
        }

        if draft.oauth_provider.is_none() || draft.oauth_source.is_none() {
            return Err(BackendError::validation("OAuth provider and source are required"));
        }

        if draft.access_token.trim().is_empty() && draft.refresh_token.trim().is_empty() {
            return Err(BackendError::validation("Authorize the OAuth account before testing or saving"));
        }

        return Ok(());
    }

    if draft.email.trim().is_empty()
        || draft.incoming_host.trim().is_empty()
        || draft.outgoing_host.trim().is_empty()
        || draft.username.trim().is_empty()
    {
        return Err(BackendError::validation("All account fields are required"));
    }

    if draft.incoming_port == 0 || draft.outgoing_port == 0 {
        return Err(BackendError::validation("Ports must be greater than 0"));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// MIME parsing helpers
// ---------------------------------------------------------------------------

pub fn extract_body_from_mime(raw: &[u8]) -> String {
    let parsed = match mailparse::parse_mail(raw) {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    let mut cid_map = Vec::new();
    collect_cid_parts_recursive(&parsed, &mut cid_map);

    let html = extract_html_part(&parsed).unwrap_or_default();
    if !html.is_empty() {
        return replace_cid_references(&html, &cid_map);
    }

    let plain = extract_text_part(&parsed).unwrap_or_default();
    if !plain.is_empty() {
        return html_escape(&plain).replace('\n', "<br>");
    }

    String::new()
}

fn extract_html_part(mail: &mailparse::ParsedMail) -> Option<String> {
    let mime_type = mail.ctype.mimetype.to_lowercase();
    if mime_type == "text/html" {
        return mail.get_body().ok();
    }
    for sub in &mail.subparts {
        if let Some(html) = extract_html_part(sub) {
            return Some(html);
        }
    }
    None
}

fn extract_text_part(mail: &mailparse::ParsedMail) -> Option<String> {
    let mime_type = mail.ctype.mimetype.to_lowercase();
    if mime_type == "text/plain" {
        return mail.get_body().ok();
    }
    for sub in &mail.subparts {
        if let Some(text) = extract_text_part(sub) {
            return Some(text);
        }
    }
    None
}

fn replace_cid_references(html: &str, cid_map: &[(String, String)]) -> String {
    let mut result = html.to_string();
    for (cid, data_url) in cid_map {
        result = result.replace(&format!("cid:{cid}"), data_url);
    }
    result
}

fn collect_cid_parts_recursive(mail: &mailparse::ParsedMail, result: &mut Vec<(String, String)>) {
    let content_id = mail
        .headers
        .iter()
        .find(|h| h.get_key().eq_ignore_ascii_case("content-id"))
        .map(|h| h.get_value())
        .map(|v: String| {
            let trimmed = v.trim();
            if trimmed.starts_with('<') && trimmed.ends_with('>') {
                trimmed[1..trimmed.len() - 1].to_string()
            } else {
                trimmed.to_string()
            }
        });

    if let Some(cid) = content_id {
        let mime_type = &mail.ctype.mimetype;
        if mime_type.starts_with("image/") {
            if let Ok(raw_body) = mail.get_body_raw() {
                let b64 = base64_encode_bytes(&raw_body);
                result.push((cid, format!("data:{mime_type};base64,{b64}")));
            }
        }
    }

    for subpart in &mail.subparts {
        collect_cid_parts_recursive(subpart, result);
    }
}

pub fn extract_attachments_from_mime(raw: &[u8]) -> Vec<AttachmentMeta> {
    let parsed = match mailparse::parse_mail(raw) {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    let mut result = Vec::new();
    let mut path = Vec::new();
    collect_attachments(&parsed, &mut result, &mut path);
    result
}

fn collect_attachments(part: &mailparse::ParsedMail, result: &mut Vec<AttachmentMeta>, path: &mut Vec<usize>) {
    let mime_type = part.ctype.mimetype.to_lowercase();

    if mime_type.starts_with("multipart/") {
        for (i, sub) in part.subparts.iter().enumerate() {
            path.push(i);
            collect_attachments(sub, result, path);
            path.pop();
        }
        return;
    }

    if is_attachment_part(part) {
        let id = if path.is_empty() {
            "0".into()
        } else {
            path.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(".")
        };

        let file_name = get_attachment_filename(part)
            .unwrap_or_else(|| format!("attachment-{id}"));

        let size_bytes = part.get_body_raw().map(|b| b.len() as u64).unwrap_or(0);

        result.push(AttachmentMeta {
            id,
            file_name,
            mime_type,
            size_bytes,
        });
    }

    for (i, sub) in part.subparts.iter().enumerate() {
        path.push(i);
        collect_attachments(sub, result, path);
        path.pop();
    }
}

fn is_attachment_part(part: &mailparse::ParsedMail) -> bool {
    let mime_type = part.ctype.mimetype.to_lowercase();

    if mime_type.starts_with("multipart/") {
        return false;
    }

    let disposition = part.get_content_disposition();
    if matches!(disposition.disposition, mailparse::DispositionType::Attachment) {
        return true;
    }

    let has_content_id = part.headers.iter()
        .any(|h| h.get_key().eq_ignore_ascii_case("content-id"));
    if has_content_id && mime_type.starts_with("image/") {
        return false;
    }

    if mime_type == "text/plain" || mime_type == "text/html" {
        return false;
    }

    true
}

pub fn get_attachment_filename(part: &mailparse::ParsedMail) -> Option<String> {
    let disposition = part.get_content_disposition();
    if let Some(filename) = disposition.params.get("filename") {
        if !filename.is_empty() {
            return Some(filename.clone());
        }
    }

    if let Some(name) = part.ctype.params.get("name") {
        if !name.is_empty() {
            return Some(name.clone());
        }
    }

    None
}

pub fn find_mime_part_by_path<'a>(
    mail: &'a mailparse::ParsedMail<'a>,
    path: &str,
) -> Option<&'a mailparse::ParsedMail<'a>> {
    if path.is_empty() {
        return Some(mail);
    }

    let indices: Vec<usize> = path.split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut current = mail;
    for idx in indices {
        if idx >= current.subparts.len() {
            return None;
        }
        current = &current.subparts[idx];
    }
    Some(current)
}

pub fn make_preview(body: &str) -> String {
    let preview = strip_html_tags(body)
        .chars()
        .take(96)
        .collect::<String>()
        .replace('\n', " ")
        .replace('\r', "");
    if preview.is_empty() {
        "(No preview)".into()
    } else {
        preview
    }
}

pub fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn base64_encode_bytes(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);

    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

pub fn decode_header_value(raw: &[u8]) -> String {
    let lossy = String::from_utf8_lossy(raw).to_string();
    decode_rfc2047(&lossy)
}

fn decode_rfc2047(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut pos = 0;
    let bytes = input.as_bytes();

    while pos < bytes.len() {
        match input[pos..].find("=?") {
            None => {
                result.push_str(&input[pos..]);
                break;
            }
            Some(offset) => {
                result.push_str(&input[pos..pos + offset]);
                pos += offset;

                match decode_one_encoded_word(&input[pos..]) {
                    Some((decoded, consumed)) => {
                        result.push_str(&decoded);
                        pos += consumed;

                        let after = &input[pos..];
                        let trimmed = after.trim_start_matches(|c: char| c == ' ' || c == '\t' || c == '\r' || c == '\n');
                        if trimmed.starts_with("=?") {
                            pos = input.len() - trimmed.len();
                        }
                    }
                    None => {
                        result.push_str("=?");
                        pos += 2;
                    }
                }
            }
        }
    }

    result
}

fn decode_one_encoded_word(input: &str) -> Option<(String, usize)> {
    if !input.starts_with("=?") {
        return None;
    }

    let rest = &input[2..];
    let q1 = rest.find('?')?;
    let charset = &rest[..q1];

    let rest = &rest[q1 + 1..];
    let q2 = rest.find('?')?;
    let encoding = &rest[..q2];

    let rest = &rest[q2 + 1..];
    let end = rest.find("?=")?;
    let encoded_text = &rest[..end];

    let consumed = 2 + q1 + 1 + q2 + 1 + end + 2;

    let decoded_bytes = match encoding.to_uppercase().as_str() {
        "B" => base64_decode(encoded_text)?,
        "Q" => qp_decode_rfc2047(encoded_text),
        _ => return None,
    };

    let decoded_str = charset_decode(&decoded_bytes, charset);
    Some((decoded_str, consumed))
}

pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let input = input.replace(['\r', '\n', ' '], "");
    let table: Vec<u8> = (0..256)
        .map(|i| match i as u8 as char {
            'A'..='Z' => (i - b'A' as usize) as u8,
            'a'..='z' => (i - b'a' as usize + 26) as u8,
            '0'..='9' => (i - b'0' as usize + 52) as u8,
            '+' => 62,
            '/' => 63,
            _ => 0xFF,
        })
        .collect();

    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();

    for chunk in bytes.chunks(4) {
        let vals: Vec<u8> = chunk.iter().map(|&b| table[b as usize]).collect();
        if vals.iter().any(|&v| v == 0xFF) {
            return None;
        }
        if vals.len() >= 2 {
            out.push((vals[0] << 2) | (vals[1] >> 4));
        }
        if vals.len() >= 3 {
            out.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if vals.len() >= 4 {
            out.push((vals[2] << 6) | vals[3]);
        }
    }

    Some(out)
}

fn qp_decode_rfc2047(input: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'_' {
            out.push(b' ');
            i += 1;
        } else if bytes[i] == b'=' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                &String::from_utf8_lossy(&bytes[i + 1..i + 3]),
                16,
            ) {
                out.push(byte);
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn charset_decode(bytes: &[u8], charset: &str) -> String {
    let charset_lower = charset.to_lowercase();
    if charset_lower == "utf-8" || charset_lower == "us-ascii" {
        return String::from_utf8_lossy(bytes).to_string();
    }
    String::from_utf8_lossy(bytes).to_string()
}
