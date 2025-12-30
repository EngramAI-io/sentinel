use clap::{Args, Parser, Subcommand};
use std::process;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;
use std::path::Path;
use std::collections::VecDeque;

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
mod audit_crypto;
mod config;
mod frontend;

use parser::Parser as LogParser;
use proxy::run_proxy;
use server::{start_server, ServerState};
use session::Session;

#[derive(Parser)]
#[command(name = "sentinel")]
#[command(about = "Secure audit logging for MCP servers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run(RunArgs),
    Verify(VerifyArgs),
    Keygen(KeygenArgs),
    RecipientKeygen(RecipientKeygenArgs),
}

#[derive(Args)]
struct RunArgs {
    #[arg(num_args = 1.., last = true)]
    command: Vec<String>,

    #[arg(long, default_value = "sentinel_audit.jsonl")]
    audit_log: String,

    #[arg(long)]
    signing_key_b64_path: Option<String>,

    #[arg(long)]
    encrypt_recipient_pubkey_b64_path: Option<String>,

    #[arg(long, default_value_t = 1000)]
    checkpoint_every: u64,

    #[arg(long, default_value = "127.0.0.1:3000")]
    ws_bind: String,

    #[arg(long)]
    ws_token: Option<String>,
}

#[derive(Args)]
struct VerifyArgs {
    #[arg(long)]
    log: String,

    #[arg(long)]
    pubkey_b64_path: String,

    #[arg(long)]
    decrypt_recipient_privkey_b64_path: Option<String>,
}

#[derive(Args)]
struct KeygenArgs {
    #[arg(long, default_value = "keys")]
    out_dir: String,
}

#[derive(Args)]
struct RecipientKeygenArgs {
    #[arg(long, default_value = "keys")]
    out_dir: String,
}

#[tokio::main]
async fn main() {
    panic::install_panic_hook();
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(a) => {
            if let Err(e) = run(a).await {
                eprintln!("❌ Fatal error: {}", e);
                process::exit(1);
            }
        }
        Commands::Verify(args) => {
            let log_path = match audit_crypto::maybe_decrypt_to_temp_plaintext(
                &args.log,
                args.decrypt_recipient_privkey_b64_path.as_deref(),
            ) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("❌ VERIFY FAILED (decryption): {}", e);
                    process::exit(2);
                }
            };

            match audit::verify_audit_log_file(
                log_path.to_string_lossy().as_ref(),
                &args.pubkey_b64_path,
            ) {
                Ok(()) => {
                    println!("✅ OK: audit log verified successfully");
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("❌ VERIFY FAILED: {}", e);
                    process::exit(2);
                }
            }
        }
        Commands::Keygen(args) => {
            if let Err(e) = keygen::generate_keypair(&args.out_dir) {
                eprintln!("❌ Key generation failed: {}", e);
                std::process::exit(1);
            }
            println!("✅ Keypair generated successfully");
            std::process::exit(0);
        }
        Commands::RecipientKeygen(args) => {
            if let Err(e) = audit_crypto::keygen_recipient(&args.out_dir) {
                eprintln!("❌ Recipient key generation failed: {}", e);
                std::process::exit(1);
            }
            println!("✅ Recipient keypair generated successfully");
            std::process::exit(0);
        }
    }
}

/// Read the first checkpoint from an existing audit log to extract key_id
fn read_first_checkpoint(log_path: &Path) -> Result<audit::AuditRecord, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    
    let file = File::open(log_path)?;
    let reader = BufReader::new(file);
    
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        
        let record: audit::AuditRecord = serde_json::from_str(&line)?;
        
        if matches!(record, audit::AuditRecord::Checkpoint { .. }) {
            return Ok(record);
        }
    }
    
    Err("No checkpoint found in existing audit log".into())
}

async fn run(args: RunArgs) -> Result<(), Box<dyn std::error::Error>> {
    let ws_token = args.ws_token
        .or_else(|| std::env::var("SENTINEL_WS_TOKEN").ok());

    let run_id = Uuid::new_v4().to_string();

    eprintln!("🚀 Starting Sentinel");
    eprintln!("   Run ID: {}", run_id);
    eprintln!("   Audit log: {}", args.audit_log);

    let signing_key = if let Some(ref key_path) = args.signing_key_b64_path {
        Some(audit::load_signing_key_b64(key_path)?)
    } else {
        eprintln!("⚠️  No signing key provided - audit log will NOT be tamper-evident");
        eprintln!("   Use --signing-key-b64-path to enable signed checkpoints");
        eprintln!("   Run 'sentinel keygen' to generate a keypair");
        None
    };

    let audit_path = Path::new(&args.audit_log);
    if let Some(ref sk) = signing_key {
        if audit_path.exists() && audit_path.metadata()?.len() > 0 {
            eprintln!("📋 Existing audit log found, validating signing key...");
            
            match read_first_checkpoint(audit_path) {
                Ok(audit::AuditRecord::Checkpoint { key_id: existing_key_id, .. }) => {
                    let current_key_id = audit::key_id_from_pubkey(&sk.verifying_key());
                    
                    if existing_key_id != current_key_id {
                        return Err(format!(
                            "Signing key mismatch!\n\
                             Existing log uses key_id: {}\n\
                             Current key has key_id: {}\n\
                             Cannot append to log with different signing key.\n\
                             Either use the original key or start a new audit log.",
                            existing_key_id,
                            current_key_id
                        ).into());
                    }
                    
                    eprintln!("   ✓ Signing key matches (key_id: {})", current_key_id);
                }
                Ok(_) => {
                    eprintln!("   ⚠️  Warning: Existing log has no checkpoint, cannot validate key");
                }
                Err(e) => {
                    eprintln!("   ⚠️  Warning: Could not read existing log: {}", e);
                    eprintln!("   Proceeding anyway (will truncate log)");
                }
            }
        }
    }

    let enable_redaction = std::env::var("SENTINEL_REDACT_PII")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    
    if enable_redaction {
        eprintln!("🔒 PII redaction enabled");
        eprintln!("   Set SENTINEL_REDACT_PII=false to disable");
    } else {
        eprintln!("⚠️  PII redaction DISABLED");
    }

    let (raw_tx, raw_rx) = mpsc::channel::<events::RawTap>(1000);
    let (tap_tx, tap_rx) = mpsc::channel::<events::TapEvent>(1000);
    let (log_tx, mut log_rx) = mpsc::channel::<events::McpLog>(1000);

    let log_tx_clone = log_tx.clone();

    let (ws_tx, _) = broadcast::channel::<events::McpLog>(1000);
    let ws_tx_for_audit = ws_tx.clone();

    let state = Arc::new(ServerState {
        tx: ws_tx.clone(),
        auth_token: ws_token.clone(),
        history: RwLock::new(VecDeque::new()),
    });

    // Assign event IDs
    tokio::spawn(async move {
        let mut id = 1u64;
        let mut rx = raw_rx;

        while let Some(r) = rx.recv().await {
            if tap_tx
                .send(events::TapEvent {
                    event_id: id,
                    direction: r.direction,
                    bytes: r.bytes,
                    observed_ts_ms: r.observed_ts_ms,
                })
                .await
                .is_err()
            {
                break;
            }
            id += 1;
        }
    });

    let session = Arc::new(Session {
        session_id: Uuid::new_v4().to_string(),
        trace_id: Uuid::new_v4().to_string(),
    });

    let run_id_clone = run_id.clone();

    // Parser
    tokio::spawn(async move {
        if let Err(e) =
            LogParser::new(run_id_clone, log_tx_clone, session)
                .process_stream(tap_rx)
                .await
        {
            eprintln!("❌ Parser error: {}", e);
        }
    });

    let audit_log_path = args.audit_log.clone();
    let encrypt_path = args.encrypt_recipient_pubkey_b64_path.clone();
    let checkpoint_every = args.checkpoint_every;
    let state_for_audit = state.clone();

    let (audit_shutdown_tx, mut audit_shutdown_rx) = mpsc::channel::<()>(1);

    // Audit + history + broadcast
    let audit_handle = tokio::spawn(async move {
        let mut file = match tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&audit_log_path)
            .await
        {
            Ok(f) => f,
            Err(e) => {
                eprintln!("❌ Failed to open audit log: {}", e);
                return;
            }
        };

        let mut sink = match audit_crypto::AuditSink::new(
            &mut file,
            &run_id,
            encrypt_path.as_deref(),
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("❌ Failed to initialize audit sink: {}", e);
                return;
            }
        };

        let mut prev_hash = [0u8; 32];
        let mut since_last_checkpoint = 0;
        let mut last_event_id = 0u64;

        loop {
            let maybe_log = tokio::select! {
                log = log_rx.recv() => log,
                _ = audit_shutdown_rx.recv() => {
                    eprintln!("🔒 Audit loop received shutdown signal");
                    None
                }
            };

            let mut log = match maybe_log {
                Some(l) => l,
                None => break,
            };

            if enable_redaction {
                redaction::redact_log(&mut log);
            }

            let (rec, hash) = match audit::make_event_record(&prev_hash, log.clone()) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("❌ Failed to create event record: {}", e);
                    continue;
                }
            };

            let rec_json = match serde_json::to_string(&rec) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("❌ Failed to serialize event record: {}", e);
                    continue;
                }
            };

            if let Err(e) = sink.write_record("Event", &rec_json).await {
                eprintln!("❌ Failed to write event record: {}", e);
                continue;
            }

            prev_hash = hash;
            last_event_id = log.event_id;
            since_last_checkpoint += 1;

            if signing_key.is_some() && since_last_checkpoint >= checkpoint_every {
                let cp = audit::make_checkpoint_record(
                    signing_key.as_ref().unwrap(),
                    &run_id,
                    events::current_timestamp_ms(),
                    last_event_id,
                    &prev_hash,
                );

                let cp_json = match serde_json::to_string(&cp) {
                    Ok(j) => j,
                    Err(e) => {
                        eprintln!("❌ Failed to serialize checkpoint: {}", e);
                        since_last_checkpoint = 0;
                        continue;
                    }
                };

                if let Err(e) = sink.write_record("Checkpoint", &cp_json).await {
                    eprintln!("❌ Failed to write checkpoint: {}", e);
                }

                since_last_checkpoint = 0;
            }

            {
                let mut hist = state_for_audit.history.write().await;
                hist.push_back(log.clone());
                if hist.len() > 10_000 {
                    hist.pop_front();
                }
            }

            let _ = ws_tx_for_audit.send(log);
        }

        if let Some(ref sk) = signing_key {
            if last_event_id > 0 {
                eprintln!("🔒 Writing final checkpoint for event_id {}", last_event_id);
                
                let final_cp = audit::make_checkpoint_record(
                    sk,
                    &run_id,
                    events::current_timestamp_ms(),
                    last_event_id,
                    &prev_hash,
                );

                if let Ok(cp_json) = serde_json::to_string(&final_cp) {
                    if let Err(e) = sink.write_record("Checkpoint", &cp_json).await {
                        eprintln!("❌ Failed to write final checkpoint: {}", e);
                    } else {
                        eprintln!("✓ Final checkpoint written");
                    }
                }
            }
        }

        if let Err(e) = sink.flush().await {
            eprintln!("❌ Failed to flush audit log: {}", e);
        } else {
            eprintln!("✓ Audit log closed cleanly");
        }
    });

    let ws_bind = args.ws_bind.clone();
    let state_for_server = state.clone();

    tokio::spawn(async move {
        if let Err(e) = start_server(state_for_server, &ws_bind).await {
            eprintln!("❌ WebSocket server error: {}", e);
        }
    });

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            eprintln!("❌ Error setting up Ctrl+C handler: {}", e);
            return;
        }
        eprintln!("\n🛑 Received Ctrl+C, shutting down gracefully...");
        let _ = shutdown_tx.send(()).await;
    });

    tokio::select! {
        result = run_proxy(args.command, raw_tx) => {
            match result {
                Ok(_) => eprintln!("📋 Proxy completed successfully"),
                Err(e) => eprintln!("❌ Proxy error: {}", e),
            }
        }
        _ = shutdown_rx.recv() => {
            eprintln!("📋 Shutdown signal received");
        }
    }

    drop(log_tx);
    if let Err(e) = audit_shutdown_tx.send(()).await {
        eprintln!("⚠️  Failed to signal audit shutdown: {}", e);
    }

    eprintln!("⏳ Waiting for audit log to finalize...");
    if let Err(e) = audit_handle.await {
        eprintln!("⚠️  Audit task join error: {}", e);
    }

    eprintln!("✅ Sentinel shutdown complete");
    Ok(())
}