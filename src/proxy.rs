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

    // Spawn child process
    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    // Child stdin (we'll write to this)
    let mut child_stdin = child
        .stdin
        .take()
        .ok_or("Failed to open child stdin")?;

    // Child stdout (we'll read from this)
    let child_stdout = child
        .stdout
        .take()
        .ok_or("Failed to open child stdout")?;

    // Parent stdin/stdout
    let parent_stdin = tokio::io::stdin();
    let mut parent_stdout = tokio::io::stdout();

    // Task: parent stdin -> child stdin (Outbound) + tap (by line)
    let tx_out = raw_sender.clone();
    let stdin_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(parent_stdin);
        let mut line = Vec::<u8>::new();

        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // Forward to child stdin immediately (never block execution on logging)
                    if let Err(e) = child_stdin.write_all(&line).await {
                        eprintln!("Error writing to child stdin: {}", e);
                        break;
                    }
                    let _ = child_stdin.flush().await;

                    // Tap (non-blocking)
                    let observed_ts_ms = current_timestamp_ms();
                    let data = Bytes::copy_from_slice(&line);
                    let _ = tx_out.try_send(RawTap {
                        direction: StreamDirection::Outbound,
                        bytes: data,
                        observed_ts_ms,
                    });
                }
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        let _ = child_stdin.shutdown().await;
    });

    // Task: child stdout -> parent stdout (Inbound) + tap (by line)
    let tx_in = raw_sender.clone();
    let stdout_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(child_stdout);
        let mut line = Vec::<u8>::new();

        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // Tap (non-blocking)
                    let observed_ts_ms = current_timestamp_ms();
                    let data = Bytes::copy_from_slice(&line);
                    let _ = tx_in.try_send(RawTap {
                        direction: StreamDirection::Inbound,
                        bytes: data.clone(),
                        observed_ts_ms,
                    });

                    // Forward to parent stdout
                    if let Err(e) = parent_stdout.write_all(&line).await {
                        eprintln!("Error writing to stdout: {}", e);
                        break;
                    }
                    let _ = parent_stdout.flush().await;
                }
                Err(e) => {
                    eprintln!("Error reading from child stdout: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for both proxy tasks to finish
    let _ = tokio::join!(stdin_handle, stdout_handle);

    // Wait for child to exit
    let status = child.wait().await?;

    // Exit with child's exit code
    process::exit(status.code().unwrap_or(1));
}
