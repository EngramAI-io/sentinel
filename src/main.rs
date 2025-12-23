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
mod audit;
mod keygen;

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

    /// Verify an audit JSONL file (hash chain + checkpoint signatures)
    Verify(VerifyArgs),

    /// Generate a new Ed25519 signing pair (base64-encoded)
    Keygen(KeygenArgs),
}

#[derive(Args)]
struct RunArgs {
    /// Command and arguments to run (everything after --)
    #[arg(num_args = 1.., last = true)]
    command: Vec<String>,

    /// Audit log output path (JSONL)
    #[arg(long, default_value = "sentinel_audit.jsonl")]
    audit_log: String,

    /// Base64(32-byte Ed25519 seed) used to sign checkpoints
    #[arg(long)]
    signing_key_b64_path: String,

    /// Emit a signed checkpoint every N events
    #[arg(long, default_value_t = 1000)]
    checkpoint_every: u64,
}

#[derive(Args)]
struct VerifyArgs {
    /// Audit log JSONL path
    #[arg(long)]
    log: String,

    /// Base64(32-byte Ed25519 public key) path
    #[arg(long)]
    pubkey_b64_path: String,
}

#[derive(Args)]
struct KeygenArgs {
    /// Output directory for key files
    #[arg(long, default_value = "keys")]
    out_dir: String,
}

#[tokio::main]
async fn main() {
    // Install panic hook early
    panic::install_panic_hook();

    let cli = Cli::parse();

    match cli.command {

        Commands::Keygen(args) => {
            if let Err(e) = keygen::generate_keypair(&args.out_dir) {
            eprintln!("Key generation failed: {}", e);
                std::process::exit(1);
            }
            std::process::exit(0);
        }
        Commands::Verify(args) => {
            match audit::verify_audit_log_file(&args.log, &args.pubkey_b64_path) {
                Ok(()) => {
                    println!("OK: audit log verified successfully");
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("VERIFY FAILED: {e}");
                    process::exit(2);
                }
            }
        }

        Commands::Run(args) => {
            if args.command.is_empty() {
                eprintln!("Error: No command provided after '--'");
                process::exit(1);
            }

            let run_id = Uuid::new_v4().to_string();
            println!("Sentinel run_id = {}", run_id);

            // Load signing key for checkpointing
            let signing_key = match audit::load_signing_key_b64(&args.signing_key_b64_path) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error: failed to load signing key: {e}");
                    process::exit(1);
                }
            };

            // Create channels for tapping and logging
            // raw taps -> sequencer -> parser logs
            let (raw_tx, mut raw_rx) = mpsc::channel::<events::RawTap>(1000);
            let (tap_tx, tap_rx) = mpsc::channel::<events::TapEvent>(1000);

            let (log_tx, mut log_rx) = mpsc::channel::<events::McpLog>(1000);

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
            let parser = LogParser::new(run_id.clone(), log_tx.clone(), session);
            let parser_handle = tokio::spawn(async move {
                if let Err(e) = parser.process_stream(tap_rx).await {
                    eprintln!("Parser error: {}", e);
                }
            });

            // Start audit log writer + WebSocket broadcaster
            let ws_tx_broadcast = ws_tx.clone();
            let audit_path = args.audit_log.clone();
            let checkpoint_every = args.checkpoint_every;
            let log_writer_handle = tokio::spawn(async move {
                let mut file = match tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&audit_path)
                    .await
                {
                    Ok(f) => tokio::io::BufWriter::new(f),
                    Err(e) => {
                        eprintln!("Warning: Failed to open audit log file {}: {}", audit_path, e);
                        return;
                    }
                };

                // Hash chain state
                let mut prev_hash: [u8; 32] = [0u8; 32];
                let mut since_checkpoint: u64 = 0;
                let mut last_event_id: u64 = 0;

                while let Some(log) = log_rx.recv().await {
                    // 1) Write event record (hash chained)
                    let (event_rec, new_hash) = match audit::make_event_record(&prev_hash, log.clone()) {
                        Ok(x) => x,
                        Err(e) => {
                            eprintln!("Warning: Failed to build audit event record: {}", e);
                            continue;
                        }
                    };

                    let json = match serde_json::to_string(&event_rec) {
                        Ok(j) => j,
                        Err(e) => {
                            eprintln!("Warning: Failed to serialize audit event record: {}", e);
                            continue;
                        }
                    };

                    if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                        &mut file,
                        format!("{}\n", json).as_bytes(),
                    )
                    .await
                    {
                        eprintln!("Warning: Failed to write audit event record: {}", e);
                    }

                    let _ = file.flush().await;

                    // advance chain
                    prev_hash = new_hash;
                    since_checkpoint += 1;
                    last_event_id = log.event_id;

                    // 2) Broadcast plain McpLog to WebSocket clients
                    let _ = ws_tx_broadcast.send(log);

                    // 3) Periodic signed checkpoints (Pattern B)
                    if checkpoint_every > 0 && since_checkpoint >= checkpoint_every {
                        let created_ts_ms = events::current_timestamp_ms();
                        let cp = audit::make_checkpoint_record(
                            &signing_key,
                            &run_id,
                            created_ts_ms,
                            last_event_id,
                            &prev_hash,
                        );

                        let cp_json = match serde_json::to_string(&cp) {
                            Ok(j) => j,
                            Err(e) => {
                                eprintln!("Warning: Failed to serialize checkpoint: {}", e);
                                since_checkpoint = 0;
                                continue;
                            }
                        };

                        if let Err(e) = tokio::io::AsyncWriteExt::write_all(
                            &mut file,
                            format!("{}\n", cp_json).as_bytes(),
                        )
                        .await
                        {
                            eprintln!("Warning: Failed to write checkpoint: {}", e);
                        }

                        let _ = file.flush().await;
                        since_checkpoint = 0;
                    }
                }

                // Optional: seal at shutdown (final checkpoint)
                if last_event_id > 0 {
                    let created_ts_ms = events::current_timestamp_ms();
                    let cp = audit::make_checkpoint_record(
                        &signing_key,
                        &run_id,
                        created_ts_ms,
                        last_event_id,
                        &prev_hash,
                    );

                    if let Ok(cp_json) = serde_json::to_string(&cp) {
                        let _ = tokio::io::AsyncWriteExt::write_all(
                            &mut file,
                            format!("{}\n", cp_json).as_bytes(),
                        )
                        .await;
                        let _ = file.flush().await;
                    }
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
