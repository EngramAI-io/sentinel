use clap::{Args, Parser, Subcommand};
use std::process;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
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

use parser::Parser as LogParser;
use proxy::run_proxy;
use server::start_server;
use session::Session;

#[derive(Parser)]
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
        Commands::RecipientKeygen(a) => {
            audit_crypto::keygen_recipient(a.out_dir).unwrap();
        }
        Commands::Keygen(a) => {
            keygen::generate_keypair(&a.out_dir).unwrap();
        }
        Commands::Verify(args) => {
    let log_path = match audit_crypto::maybe_decrypt_to_temp_plaintext(
        &args.log,
        args.decrypt_recipient_privkey_b64_path.as_deref(),
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("VERIFY FAILED: {e}");
            process::exit(2);
        }
    };

    match audit::verify_audit_log_file(
        log_path.to_string_lossy().as_ref(),
        &args.pubkey_b64_path,
    ) {
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

        Commands::Run(a) => run(a).await,
    }
}

async fn run(args: RunArgs) {
    let run_id = Uuid::new_v4().to_string();
    let signing_key = audit::load_signing_key_b64(&args.signing_key_b64_path).unwrap();

    let (raw_tx, raw_rx) = mpsc::channel::<events::RawTap>(1000);
    let (tap_tx, tap_rx) = mpsc::channel::<events::TapEvent>(1000);
    let (log_tx, mut log_rx) = mpsc::channel::<events::McpLog>(1000);
    let (ws_tx, _) = broadcast::channel::<events::McpLog>(1000);

    tokio::spawn(async move {
    let mut rx = raw_rx;
    let mut id = 1u64;

    while let Some(r) = rx.recv().await {
        tap_tx.send(events::TapEvent {
            event_id: id,
            direction: r.direction,
            bytes: r.bytes,
            observed_ts_ms: r.observed_ts_ms,
        }).await.unwrap();
        id += 1;
    }
});


    let session = Arc::new(Session {
        session_id: Uuid::new_v4().to_string(),
        trace_id: Uuid::new_v4().to_string(),
    });

    let run_id_clone = run_id.clone();

    tokio::spawn(async move {
        LogParser::new(run_id_clone, log_tx, session)
            .process_stream(tap_rx)
            .await
            .unwrap();
    });

    let audit_path = args.audit_log.clone();
    let encrypt_path = args.encrypt_recipient_pubkey_b64_path.clone();
    let checkpoint_every = args.checkpoint_every;

    tokio::spawn(async move {
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
        ).await.unwrap();

        let mut prev_hash = [0u8; 32];
        let mut since = 0;
        let mut last = 0;

        while let Some(log) = log_rx.recv().await {
            let (rec, hash) = audit::make_event_record(&prev_hash, log.clone()).unwrap();
            let json = serde_json::to_string(&rec).unwrap();
            sink.write_record("Event", &json).await.unwrap();

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
                sink.write_record("Checkpoint", &serde_json::to_string(&cp).unwrap())
                    .await.unwrap();
                since = 0;
            }

            let _ = ws_tx.send(log);
        }

        sink.flush().await.unwrap();
    });

    run_proxy(args.command, raw_tx).await.unwrap();
}
