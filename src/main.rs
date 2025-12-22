use clap::{Args, Parser as ClapParser, Subcommand};
use std::process;
use std::sync::Arc;
use session::Session;
use tokio::sync::{broadcast, mpsc};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

mod proxy;
mod protocol;
mod events;
mod parser;
mod session;
mod server;
mod redaction;
mod panic;

use events::StreamDirection;
use parser::Parser as LogParser;
use proxy::run_proxy;
use server::start_server;

#[derive(ClapParser)]
#[command(name = "sentinel")]
#[command(about = "MCP Interceptor - Transparent proxy for Model Context Protocol")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a command through the proxy
    Run(RunArgs),
}

#[derive(Args)]
struct RunArgs {
    /// Command and arguments to run (everything after --)
    #[arg(num_args = 1.., last = true)]
    command: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Install panic hook early
    panic::install_panic_hook();

    let cli = Cli::parse();

    let run_id = Uuid::new_v4().to_string();
    println!("Sentinel run_id = {}", run_id);

    match cli.command {
        Commands::Run(args) => {
            if args.command.is_empty() {
                eprintln!("Error: No command provided after '--'");
                process::exit(1);
            }

            // Create channels for tapping and logging
            // Create channels: raw taps -> sequencer -> parser logs
            let (raw_tx, mut raw_rx) = mpsc::channel::<events::RawTap>(1000);
            let (tap_tx, tap_rx) = mpsc::channel::<events::TapEvent>(1000);

            let (log_tx, mut log_rx) = mpsc::channel::<events::McpLog>(1000);
            let log_tx_clone = log_tx.clone();

            // Sequencer task: assigns canonical event_id in a single place
            let sequencer_handle = tokio::spawn(async move {
                let mut next_id: u64 = 1;
                while let Some(raw) = raw_rx.recv().await {
                    let evt = events::TapEvent {
                        event_id: next_id,
                        direction: raw.direction,
                        bytes: raw.bytes,
                        observed_ts_ms: raw.observed_ts_ms,
                    };
                    next_id += 1;

                    // If parser is gone, stop sequencing
                    if tap_tx.send(evt).await.is_err() {
                        break;
                    }
                }
            });

            let log_tx_clone = log_tx.clone();

            // Broadcast channel for WebSocket clients
            let (ws_tx, _) = broadcast::channel::<events::McpLog>(1000);

            // Start HTTP/WebSocket server
            let ws_tx_server = ws_tx.clone();
            let server_handle = tokio::spawn(async move {
                if let Err(e) = start_server(ws_tx_server).await {
                    eprintln!("Server error: {}", e);
                }
            });
            
            
            let session = Arc::new(Session {
            session_id: Uuid::new_v4().to_string(),
            trace_id: Uuid::new_v4().to_string(),
        });

            // Start parser
            let parser = LogParser::new(run_id.clone(), log_tx_clone, session);
            let parser_handle = tokio::spawn(async move {
                if let Err(e) = parser.process_stream(tap_rx).await {
                    eprintln!("Parser error: {}", e);
                }
            });

            // Start log writer task and WebSocket broadcaster
            let ws_tx_broadcast = ws_tx.clone();
            let log_writer_handle = tokio::spawn(async move {
                let mut file = match tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("sentinel_debug.jsonl")
                    .await
                {
                    Ok(f) => tokio::io::BufWriter::new(f),
                    Err(e) => {
                        eprintln!("Warning: Failed to open log file: {}", e);
                        return;
                    }
                };

                while let Some(log) = log_rx.recv().await {
                    let json = match serde_json::to_string(&log) {
                        Ok(j) => j,
                        Err(e) => {
                            eprintln!("Warning: Failed to serialize log: {}", e);
                            continue;
                        }
                    };

                    if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                        &mut file,
                        format!("{}\n", json).as_bytes(),
                    )
                    .await
                    {
                        eprintln!("Warning: Failed to write log: {}", e);
                    }

                    let _ = file.flush().await;

                    // Broadcast to WebSocket clients (ignore errors if no clients)
                    let _ = ws_tx_broadcast.send(log);
                }
            });

            // Run the proxy in the current task
            let command = args.command;
            if let Err(e) = run_proxy(command, raw_tx).await {
                eprintln!("Proxy error: {}", e);
                process::exit(1);
            }

            // If proxy exits, shut down parser/log/server
            drop(log_tx); // close log channel
            let _ = parser_handle.abort();
            let _ = log_writer_handle.abort();
            let _ = server_handle.abort();
            let _ = sequencer_handle.abort();
        }
    }
}
