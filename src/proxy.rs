use crate::events::{current_timestamp_ms, RawTap, StreamDirection};
use bytes::Bytes;
use std::process::{self, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

pub async fn run_proxy(
    command: Vec<String>,
    raw_sender: mpsc::Sender<RawTap>,
) -> Result<(), Box<dyn std::error::Error>> {
    if command.is_empty() {
        return Err("Empty command".into());
    }

    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let mut child_stdin = child.stdin.take().ok_or("Failed to open child stdin")?;
    let child_stdout = child.stdout.take().ok_or("Failed to open child stdout")?;

    let parent_stdin = tokio::io::stdin();
    let mut parent_stdout = tokio::io::stdout();

    // ----- OUTBOUND: parent stdin -> child stdin -----
    let tx_out = raw_sender.clone();
    let stdin_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(parent_stdin);
        let mut line = Vec::<u8>::new();

        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    // Forward FIRST
                    if child_stdin.write_all(&line).await.is_err() {
                        break;
                    }
                    let _ = child_stdin.flush().await;

                    // FIX: lossless tap
                    let observed_ts_ms = current_timestamp_ms();
                    let data = Bytes::copy_from_slice(&line);
                    if tx_out
                        .send(RawTap {
                            direction: StreamDirection::Outbound,
                            bytes: data,
                            observed_ts_ms,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        let _ = child_stdin.shutdown().await;
    });

    // ----- INBOUND: child stdout -> parent stdout -----
    let tx_in = raw_sender.clone();
    let stdout_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(child_stdout);
        let mut line = Vec::<u8>::new();

        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    // Forward FIRST
                    if parent_stdout.write_all(&line).await.is_err() {
                        break;
                    }
                    let _ = parent_stdout.flush().await;

                    let observed_ts_ms = current_timestamp_ms();
                    let data = Bytes::copy_from_slice(&line);
                    if tx_in
                        .send(RawTap {
                            direction: StreamDirection::Inbound,
                            bytes: data,
                            observed_ts_ms,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = tokio::join!(stdin_handle, stdout_handle);
    let status = child.wait().await?;
    process::exit(status.code().unwrap_or(1));
}
