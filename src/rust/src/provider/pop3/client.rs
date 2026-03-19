use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use crate::models::{AccountAuthMode, AccountSetupDraft, StoredAccountState};
use crate::protocol::BackendError;
use crate::provider::common::redact_email_for_log;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

enum Pop3Stream {
    Plain(BufReader<TcpStream>),
    Tls(BufReader<TlsStream<TcpStream>>),
}

pub(super) struct Pop3Client {
    stream: Pop3Stream,
}

#[derive(Debug)]
pub(super) struct Pop3Stat {
    pub count: usize,
    pub _size: usize,
}

impl Pop3Client {
    pub(super) async fn connect_plain(host: &str, port: u16) -> Result<Self, BackendError> {
        let tcp = tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect((host, port)))
            .await
            .map_err(|_| BackendError::validation("POP3 connection timed out"))?
            .map_err(|e| BackendError::validation(format!("POP3 connection failed: {e}")))?;

        let mut client = Self {
            stream: Pop3Stream::Plain(BufReader::new(tcp)),
        };

        client.read_greeting().await?;
        Ok(client)
    }

    pub(super) async fn connect_tls(host: &str, port: u16) -> Result<Self, BackendError> {
        let tcp = tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect((host, port)))
            .await
            .map_err(|_| BackendError::validation("POP3 connection timed out"))?
            .map_err(|e| BackendError::validation(format!("POP3 connection failed: {e}")))?;

        let connector = native_tls::TlsConnector::new()
            .map_err(|e| BackendError::validation(format!("TLS error: {e}")))?;
        let connector = tokio_native_tls::TlsConnector::from(connector);
        let tls = connector
            .connect(host, tcp)
            .await
            .map_err(|e| BackendError::validation(format!("TLS handshake failed: {e}")))?;

        let mut client = Self {
            stream: Pop3Stream::Tls(BufReader::new(tls)),
        };

        client.read_greeting().await?;
        Ok(client)
    }

    pub(super) async fn connect_from_state(state: &StoredAccountState) -> Result<Self, BackendError> {
        let host = state.config.incoming_host.trim();
        let port = state.config.incoming_port;

        if state.config.use_tls {
            Self::connect_tls(host, port).await
        } else {
            Self::connect_plain(host, port).await
        }
    }

    pub(super) async fn login_test(draft: &AccountSetupDraft) -> Result<(), BackendError> {
        if matches!(draft.auth_mode, AccountAuthMode::Oauth) {
            return Err(BackendError::validation(
                "POP3 does not support OAuth accounts in MailYou",
            ));
        }

        let host = draft.incoming_host.trim();
        let port = draft.incoming_port;

        eprintln!(
            "[pop3] connecting to {host}:{port} (tls={})...",
            draft.use_tls
        );

        let mut client = if draft.use_tls {
            Self::connect_tls(host, port).await?
        } else {
            Self::connect_plain(host, port).await?
        };

        eprintln!(
            "[pop3] logging in as {}...",
            redact_email_for_log(draft.username.trim())
        );
        client
            .login(draft.username.trim(), draft.password.trim())
            .await?;
        client.quit().await?;

        Ok(())
    }

    async fn read_greeting(&mut self) -> Result<(), BackendError> {
        let line = self.read_line().await?;
        if !line.starts_with("+OK") {
            return Err(BackendError::validation(format!(
                "POP3 greeting failed: {line}"
            )));
        }
        Ok(())
    }

    async fn read_line(&mut self) -> Result<String, BackendError> {
        let mut line = String::new();
        match &mut self.stream {
            Pop3Stream::Plain(reader) => {
                reader
                    .read_line(&mut line)
                    .await
                    .map_err(|e| BackendError::internal(format!("POP3 read failed: {e}")))?;
            }
            Pop3Stream::Tls(reader) => {
                reader
                    .read_line(&mut line)
                    .await
                    .map_err(|e| BackendError::internal(format!("POP3 read failed: {e}")))?;
            }
        }
        Ok(line.trim_end().to_string())
    }

    async fn write_line(&mut self, line: &str) -> Result<(), BackendError> {
        let data = format!("{line}\r\n");
        match &mut self.stream {
            Pop3Stream::Plain(reader) => {
                reader
                    .get_mut()
                    .write_all(data.as_bytes())
                    .await
                    .map_err(|e| BackendError::internal(format!("POP3 write failed: {e}")))?;
            }
            Pop3Stream::Tls(reader) => {
                reader
                    .get_mut()
                    .write_all(data.as_bytes())
                    .await
                    .map_err(|e| BackendError::internal(format!("POP3 write failed: {e}")))?;
            }
        }
        Ok(())
    }

    async fn command(&mut self, cmd: &str) -> Result<String, BackendError> {
        self.write_line(cmd).await?;
        let response = self.read_line().await?;
        if !response.starts_with("+OK") {
            return Err(BackendError::internal(format!(
                "POP3 command failed: {response}"
            )));
        }
        Ok(response)
    }

    pub(super) async fn login(&mut self, username: &str, password: &str) -> Result<(), BackendError> {
        self.command(&format!("USER {username}")).await?;
        self.command(&format!("PASS {password}")).await?;
        Ok(())
    }

    pub(super) async fn stat(&mut self) -> Result<Pop3Stat, BackendError> {
        let response = self.command("STAT").await?;
        let parts: Vec<&str> = response.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(BackendError::internal("Invalid STAT response"));
        }
        let count = parts[1]
            .parse()
            .map_err(|_| BackendError::internal("Invalid STAT count"))?;
        let size = parts[2]
            .parse()
            .map_err(|_| BackendError::internal("Invalid STAT size"))?;
        Ok(Pop3Stat { count, _size: size })
    }

    pub(super) async fn uidl(
        &mut self,
    ) -> Result<std::collections::HashMap<usize, String>, BackendError> {
        self.write_line("UIDL").await?;
        let response = self.read_line().await?;
        if !response.starts_with("+OK") {
            return Err(BackendError::internal(format!("UIDL failed: {response}")));
        }

        let mut map = std::collections::HashMap::new();
        loop {
            let line = self.read_line().await?;
            if line == "." {
                break;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[0].parse::<usize>() {
                    map.insert(num, parts[1].to_string());
                }
            }
        }
        Ok(map)
    }

    pub(super) async fn retr(&mut self, msg_num: usize) -> Result<Vec<u8>, BackendError> {
        self.write_line(&format!("RETR {msg_num}")).await?;
        let response = self.read_line().await?;
        if !response.starts_with("+OK") {
            return Err(BackendError::internal(format!("RETR failed: {response}")));
        }

        let mut data = Vec::new();
        loop {
            let line = self.read_line().await?;
            if line == "." {
                break;
            }
            let line = if line.starts_with("..") {
                &line[1..]
            } else {
                &line
            };
            data.extend_from_slice(line.as_bytes());
            data.extend_from_slice(b"\r\n");
        }
        Ok(data)
    }

    pub(super) async fn quit(&mut self) -> Result<(), BackendError> {
        self.command("QUIT").await?;
        Ok(())
    }
}
