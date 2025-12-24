use clap::{Args, Parser, Subcommand};
use std::process;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::signal;
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
mod audit_crypto;
mod config;

use parser::Parser as LogParser;
use proxy::run_proxy;
use server::start_server;
use session::Session;

#[derive(Parser)]
#[command(name = "sentinel")]
#[command(about = "Secure audit logging for MCP servers", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run MCP server with audit logging
    Run(RunArgs),
    /// Verify audit log integrity
    Verify(VerifyArgs),
    /// Generate Ed25519 signing keypair
    Keygen(KeygenArgs),
    /// Generate X25519 encryption keypair
    RecipientKeygen(RecipientKeygenArgs),
}

#[derive(Args)]
struct RunArgs {
    /// Command and arguments to execute
    #[arg(num_args = 1.., last = true)]
    command: Vec<String>,

    /// Path to audit log file
    #[arg(long, default_value = "sentinel_audit.jsonl")]
    audit_log: String,

    /// Path to signing key (base64 encoded)
    #[arg(long)]
    signing_key_b64_path: String,

    /// Path to recipient public key for encryption (optional)
    #[arg(long)]
    encrypt_recipient_pubkey_b64_path: Option<String>,

    /// Number of events between checkpoints
    #[arg(long, default_value_t = 1000)]
    checkpoint_every: u64,

    /// WebSocket server bind address
    #[arg(long, default_value = "127.0.0.1:3000")]
    ws_bind: String,

    /// WebSocket authentication token (recommended for security)
    #[arg(long, env = "SENTINEL_WS_TOKEN")]
    ws_token: Option<String>,
}

#[derive(Args)]
struct VerifyArgs {
    /// Path to audit log file
    #[arg(long)]
    log: String,

    /// Path to public key (base64 encoded)
    #[arg(long)]
    pubkey_b64_path: String,

    /// Path to recipient private key for decryption (if log is encrypted)
    #[arg(long)]
    decrypt_recipient_privkey_b64_path: Option<String>,
}

#[derive(Args)]
struct KeygenArgs {
    /// Output directory for generated keys
    #[arg(long, default_value = "keys")]
    out_dir: String,
}

#[derive(Args)]
struct RecipientKeygenArgs {
    /// Output directory for generated keys
    #[arg(long, default_value = "keys")]
    out_dir: String,
}

#[tokio::main]
async fn main() {
    panic::install_panic_hook();
    let cli = Cli::parse();

    match cli.command {
        Commands::RecipientKeygen(a) => {
            match audit_crypto::keygen_recipient(a.out_dir) {
                Ok(()) => process::exit(0),
                Err(e) => {
                    eprintln!("❌ Failed to generate recipient keypair");
                    eprintln!("   Error: {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::Keygen(a) => {
            match keygen::generate_keypair(&a.out_dir) {
                Ok(()) => process::exit(0),
                Err(e) => {
                    eprintln!("❌ Failed to generate signing keypair");
                    eprintln!("   Error: {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::Verify(args) => {
            eprintln!("🔍 Verifying audit log: {}", args.log);
            
            let log_path = match audit_crypto::maybe_decrypt_to_temp_plaintext(
                &args.log,
                args.decrypt_recipient_privkey_b64_path.as_deref(),
            ) {
                Ok(p) => {
                    if p.to_string_lossy() != args.log {
                        eprintln!("🔓 Log is encrypted, decrypted to temporary file");
                    }
                    p
                }
                Err(e) => {
                    eprintln!("❌ Verification failed: Unable to decrypt log");
                    eprintln!("   Error: {}", e);
                    process::exit(2);
                }
            };

            match audit::verify_audit_log_file(
                log_path.to_string_lossy().as_ref(),
                &args.pubkey_b64_path,
            ) {
                Ok(()) => {
                    eprintln!("✅ Audit log verification successful");
                    eprintln!("   All signatures valid, chain integrity verified");
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("❌ Verification failed: Audit log integrity compromised");
                    eprintln!("   Error: {}", e);
                    process::exit(2);
                }
            }
        }
        Commands::Run(a) => {
            if let Err(e) = run(a).await {
                eprintln!("❌ Fatal error: {}", e);
                process::exit(1);
            }
        }
    }
}

async fn run(args: RunArgs) -> Result<(), Box<dyn std::error::Error>> {
    let run_id = Uuid::new_v4().to_string();
    
    eprintln!("🚀 Starting Sentinel");
    eprintln!("   Run ID: {}", run_id);
    eprintln!("   Audit log: {}", args.audit_log);
    
    let signing_key = match audit::load_signing_key_b64(&args.signing_key_b64_path) {
        Ok(key) => {
            eprintln!("✅ Loaded signing key");
            key
        }
        Err(e) => {
            eprintln!("❌ Failed to load signing key from {}", args.signing_key_b64_path);
            return Err(format!("Signing key error: {}", e).into());
        }
    };

    let (raw_tx, raw_rx) = mpsc::channel::<events::RawTap>(1000);
    let (tap_tx, tap_rx) = mpsc::channel::<events::TapEvent>(1000);
    let (log_tx, mut log_rx) = mpsc::channel::<events::McpLog>(1000);
    let (ws_tx, _) = broadcast::channel::<events::McpLog>(1000);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

    // Event ID assignment task
    tokio::spawn(async move {
        let mut rx = raw_rx;
        let mut id = 1u64;

        while let Some(r) = rx.recv().await {
            if tap_tx.send(events::TapEvent {
                event_id: id,
                direction: r.direction,
                bytes: r.bytes,
                observed_ts_ms: r.observed_ts_ms,
            }).await.is_err() {
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

    // Parser task
    tokio::spawn(async move {
        if let Err(e) = LogParser::new(run_id_clone, log_tx, session)
            .process_stream(tap_rx)
            .await
        {
            eprintln!("❌ Parser error: {}", e);
        }
    });

    let audit_path = args.audit_log.clone();
    let encrypt_path = args.encrypt_recipient_pubkey_b64_path.clone();
    let checkpoint_every = args.checkpoint_every;

    // Audit logging task with graceful shutdown
    let audit_handle = tokio::spawn(async move {
        let mut file = match tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&audit_path)
            .await
        {
            Ok(f) => f,
            Err(e) => {
                eprintln!("❌ Failed to open audit log: {}", e);
                return Err(format!("Audit log error: {}", e));
            }
        };

        let mut sink = match audit_crypto::AuditSink::new(
            &mut file,
            &run_id,
            encrypt_path.as_deref(),
        ).await {
            Ok(s) => {
                if encrypt_path.is_some() {
                    eprintln!("🔒 Audit log encryption enabled");
                }
                s
            }
            Err(e) => {
                eprintln!("❌ Failed to initialize audit sink: {}", e);
                return Err(e);
            }
        };

        let mut prev_hash = [0u8; 32];
        let mut since = 0;
        let mut last = 0;

        while let Some(log) = log_rx.recv().await {
            match audit::make_event_record(&prev_hash, log.clone()) {
                Ok((rec, hash)) => {
                    if let Ok(json) = serde_json::to_string(&rec) {
                        if let Err(e) = sink.write_record("Event", &json).await {
                            eprintln!("❌ Failed to write audit record: {}", e);
                            continue;
                        }
                        prev_hash = hash;
                        last = log.event_id;
                        since += 1;
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to create event record: {}", e);
                    continue;
                }
            }

            if since >= checkpoint_every {
                let cp = audit::make_checkpoint_record(
                    &signing_key,
                    &run_id,
                    events::current_timestamp_ms(),
                    last,
                    &prev_hash,
                );
                if let Ok(json) = serde_json::to_string(&cp) {
                    if let Err(e) = sink.write_record("Checkpoint", &json).await {
                        eprintln!("❌ Failed to write checkpoint: {}", e);
                    } else {
                        eprintln!("📝 Checkpoint written at event {}", last);
                    }
                }
                since = 0;
            }

            let _ = ws_tx.send(log);
        }

        eprintln!("🔄 Flushing audit log...");
        if let Err(e) = sink.flush().await {
            eprintln!("❌ Failed to flush audit log: {}", e);
            return Err(e);
        }

        eprintln!("✅ Audit log flushed successfully");
        Ok(())
    });

    // WebSocket server task
    let ws_bind = args.ws_bind.clone();
    let ws_token = args.ws_token.clone();
    let ws_handle = tokio::spawn(async move {
        if let Err(e) = start_server(ws_tx, &ws_bind, ws_token).await {
            eprintln!("❌ WebSocket server error: {}", e);
        }
    });

    // Graceful shutdown handler
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                eprintln!("\n🛑 Shutdown signal received, gracefully shutting down...");
                let _ = shutdown_tx.send(()).await;
            }
            Err(e) => {
                eprintln!("❌ Failed to listen for shutdown signal: {}", e);
            }
        }
    });

    // Run proxy until completion or shutdown
    tokio::select! {
        result = run_proxy(args.command, raw_tx) => {
            match result {
                Ok(()) => eprintln!("✅ Proxy completed successfully"),
                Err(e) => eprintln!("❌ Proxy error: {}", e),
            }
        }
        _ = shutdown_rx.recv() => {
            eprintln!("🛑 Shutdown initiated");
        }
    }

    // Wait for audit task to complete with timeout
    eprintln!("⏳ Waiting for audit log to flush (max 10s)...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        audit_handle
    ).await {
        Ok(Ok(Ok(()))) => eprintln!("✅ Audit log completed successfully"),
        Ok(Ok(Err(e))) => eprintln!("❌ Audit log error: {}", e),
        Ok(Err(e)) => eprintln!("❌ Audit task panicked: {}", e),
        Err(_) => eprintln!("⚠️  Audit log flush timed out"),
    }

    // Cleanup WebSocket server
    ws_handle.abort();
    
    eprintln!("👋 Sentinel shutdown complete");
    Ok(())
}
