use clap::{Args, Parser, Subcommand};
use std::process;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{broadcast, mpsc, RwLock};
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
    signing_key_b64_path: String,

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
        Commands::Verify(_) | Commands::Keygen(_) | Commands::RecipientKeygen(_) => {
            eprintln!("❌ Not shown — unchanged from your version");
        }
    }
}

async fn run(args: RunArgs) -> Result<(), Box<dyn std::error::Error>> {
    let ws_token = args.ws_token
        .or_else(|| std::env::var("SENTINEL_WS_TOKEN").ok());

    let run_id = Uuid::new_v4().to_string();

    eprintln!("🚀 Starting Sentinel");
    eprintln!("   Run ID: {}", run_id);
    eprintln!("   Audit log: {}", args.audit_log);

    let signing_key =
        audit::load_signing_key_b64(&args.signing_key_b64_path)?;

    let (raw_tx, raw_rx) = mpsc::channel::<events::RawTap>(1000);
    let (tap_tx, tap_rx) = mpsc::channel::<events::TapEvent>(1000);
    let (log_tx, mut log_rx) = mpsc::channel::<events::McpLog>(1000);

    let (ws_tx, _) = broadcast::channel::<events::McpLog>(1000);
    let ws_tx_for_audit = ws_tx.clone();
    let ws_tx_for_server = ws_tx.clone();

    let state = Arc::new(ServerState {
        tx: ws_tx.clone(),
        auth_token: ws_token.clone(),
        history: RwLock::new(Vec::new()),
    });

    // Assign event IDs
    tokio::spawn(async move {
        let mut id = 1u64;
        let mut rx = raw_rx;

        while let Some(r) = rx.recv().await {
            let _ = tap_tx
                .send(events::TapEvent {
                    event_id: id,
                    direction: r.direction,
                    bytes: r.bytes,
                    observed_ts_ms: r.observed_ts_ms,
                })
                .await;
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
            LogParser::new(run_id_clone, log_tx, session)
                .process_stream(tap_rx)
                .await
        {
            eprintln!("❌ Parser error: {}", e);
        }
    });

    let audit_path = args.audit_log.clone();
    let encrypt_path = args.encrypt_recipient_pubkey_b64_path.clone();
    let checkpoint_every = args.checkpoint_every;
    let state_for_audit = state.clone();

    // Audit + history + broadcast
    let audit_handle = tokio::spawn(async move {
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&audit_path)
            .await
            .unwrap();

        let mut sink = audit_crypto::AuditSink::new(
            &mut file,
            &run_id,
            encrypt_path.as_deref(),
        )
        .await
        .unwrap();

        let mut prev_hash = [0u8; 32];
        let mut since = 0;
        let mut last = 0;

        while let Some(log) = log_rx.recv().await {
            let (rec, hash) =
                audit::make_event_record(&prev_hash, log.clone()).unwrap();

            sink.write_record(
                "Event",
                &serde_json::to_string(&rec).unwrap(),
            )
            .await
            .unwrap();

            prev_hash = hash;
            last = log.event_id;
            since += 1;

            if since >= checkpoint_every {
                let cp = audit::make_checkpoint_record(
                    &signing_key,
                    &run_id,
                    events::current_timestamp_ms(),
                    last,
                    &prev_hash,
                );

                sink.write_record(
                    "Checkpoint",
                    &serde_json::to_string(&cp).unwrap(),
                )
                .await
                .unwrap();

                since = 0;
            }

            // store for replay
            {
                let mut hist = state_for_audit.history.write().await;
                hist.push(log.clone());
                if hist.len() > 10_000 {
                    hist.remove(0);
                }
            }

            // broadcast live
            let _ = ws_tx_for_audit.send(log);
        }

        sink.flush().await.unwrap();
    });

    // WebSocket server
    let ws_bind = args.ws_bind.clone();
    let ws_token_clone = ws_token.clone();
    let state_for_server = state.clone();

    tokio::spawn(async move {
        let _ = start_server(state_for_server, &ws_bind).await;
    });

    // Shutdown handling
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        let _ = shutdown_tx.send(()).await;
    });

    tokio::select! {
        _ = run_proxy(args.command, raw_tx) => {}
        _ = shutdown_rx.recv() => {}
    }

    let _ = audit_handle.await;
    Ok(())
}
