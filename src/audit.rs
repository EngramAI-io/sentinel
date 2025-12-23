use crate::events::McpLog;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use blake3;
use ed25519_dalek::{Signature, SigningKey, Signer, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub const HASH_ALG: &str = "blake3";
pub const SIG_ALG: &str = "ed25519";

/// Wrapper record written to JSONL.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "record_type")]
pub enum AuditRecord {
    Event {
        log: McpLog,
        integrity: IntegrityFields,
    },
    Checkpoint {
        run_id: String,
        created_ts_ms: u64,
        last_event_id: u64,
        last_entry_hash_b64: String,
        signature_b64: String,
        key_id: String,
        hash_alg: String,
        sig_alg: String,
        version: u32,
    },
}

/// Integrity metadata attached to each event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityFields {
    pub prev_hash_b64: String,
    pub entry_hash_b64: String,
    pub hash_alg: String,
    pub version: u32,
}

/// Deterministic subset of McpLog used for hashing.
/// payload is recursively canonicalized to sort object keys.
#[derive(Debug, Clone, Serialize)]
struct SignableMcpLog<'a> {
    run_id: &'a str,
    event_id: u64,
    observed_ts_ms: u64,
    timestamp: u64,
    direction: crate::events::StreamDirection,
    method: &'a Option<String>,
    request_id: &'a Option<u64>,
    latency_ms: &'a Option<u64>,
    payload: Value,
    session_id: &'a str,
    trace_id: &'a str,
    span_id: &'a str,
    parent_span_id: &'a Option<String>,
}

fn canonicalize_value(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for k in keys {
                if let Some(val) = map.get(&k) {
                    out.insert(k, canonicalize_value(val));
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_value).collect()),
        _ => v.clone(),
    }
}

fn signable_bytes(log: &McpLog) -> Result<Vec<u8>, String> {
    let signable = SignableMcpLog {
        run_id: &log.run_id,
        event_id: log.event_id,
        observed_ts_ms: log.observed_ts_ms,
        timestamp: log.timestamp,
        direction: log.direction,
        method: &log.method,
        request_id: &log.request_id,
        latency_ms: &log.latency_ms,
        payload: canonicalize_value(&log.payload),
        session_id: &log.session_id,
        trace_id: &log.trace_id,
        span_id: &log.span_id,
        parent_span_id: &log.parent_span_id,
    };
    serde_json::to_vec(&signable).map_err(|e| format!("failed to serialize signable log: {e}"))
}

fn decode_b64_32(s: &str) -> Result<[u8; 32], String> {
    let bytes = B64
        .decode(s)
        .map_err(|e| format!("base64 decode failed: {e}"))?;
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn encode_b64_32(b: &[u8; 32]) -> String {
    B64.encode(b)
}

fn checkpoint_preimage(run_id: &str, last_event_id: u64, last_entry_hash: &[u8; 32]) -> [u8; 32] {
    // Hash a deterministic preimage for the signature.
    // This binds the signature to a specific run + point-in-stream.
    let mut hasher = blake3::Hasher::new();
    hasher.update(run_id.as_bytes());
    hasher.update(&last_event_id.to_le_bytes());
    hasher.update(last_entry_hash);
    *hasher.finalize().as_bytes()
}

pub fn key_id_from_pubkey(pubkey: &VerifyingKey) -> String {
    // Short, stable identifier auditors can refer to.
    let bytes = pubkey.to_bytes();
    let fp = blake3::hash(&bytes);
    // 12 hex chars is enough as an identifier (not a security boundary).
    hex::encode(&fp.as_bytes()[0..6])
}

pub fn load_signing_key_b64(path: impl AsRef<Path>) -> Result<SigningKey, String> {
    // File contains base64 of 32-byte Ed25519 seed.
    let s = fs::read_to_string(path).map_err(|e| format!("failed to read key file: {e}"))?;
    let s = s.trim();
    let seed = B64
        .decode(s)
        .map_err(|e| format!("failed to base64-decode seed: {e}"))?;
    if seed.len() != 32 {
        return Err(format!("expected 32-byte seed, got {}", seed.len()));
    }
    let mut seed32 = [0u8; 32];
    seed32.copy_from_slice(&seed);
    Ok(SigningKey::from_bytes(&seed32))
}

pub fn load_verify_key_b64(path: impl AsRef<Path>) -> Result<VerifyingKey, String> {
    // File contains base64 of 32-byte Ed25519 public key.
    let s = fs::read_to_string(path).map_err(|e| format!("failed to read pubkey file: {e}"))?;
    let s = s.trim();
    let pk = B64
        .decode(s)
        .map_err(|e| format!("failed to base64-decode pubkey: {e}"))?;
    if pk.len() != 32 {
        return Err(format!("expected 32-byte public key, got {}", pk.len()));
    }
    let mut pk32 = [0u8; 32];
    pk32.copy_from_slice(&pk);
    VerifyingKey::from_bytes(&pk32).map_err(|e| format!("invalid verifying key: {e}"))
}

/// Compute entry hash = blake3(prev_hash || signable_bytes)
pub fn compute_entry_hash(prev_hash: &[u8; 32], log: &McpLog) -> Result<[u8; 32], String> {
    let bytes = signable_bytes(log)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(prev_hash);
    hasher.update(&bytes);
    Ok(*hasher.finalize().as_bytes())
}

/// Build an event record + updated prev hash.
pub fn make_event_record(
    prev_hash: &[u8; 32],
    log: McpLog,
) -> Result<(AuditRecord, [u8; 32]), String> {
    let entry_hash = compute_entry_hash(prev_hash, &log)?;
    let rec = AuditRecord::Event {
        log,
        integrity: IntegrityFields {
            prev_hash_b64: encode_b64_32(prev_hash),
            entry_hash_b64: encode_b64_32(&entry_hash),
            hash_alg: HASH_ALG.to_string(),
            version: 1,
        },
    };
    Ok((rec, entry_hash))
}

/// Build a signed checkpoint record for the current chain tip.
pub fn make_checkpoint_record(
    signing_key: &SigningKey,
    run_id: &str,
    created_ts_ms: u64,
    last_event_id: u64,
    last_entry_hash: &[u8; 32],
) -> AuditRecord {
    let pubkey = signing_key.verifying_key();
    let key_id = key_id_from_pubkey(&pubkey);

    let pre = checkpoint_preimage(run_id, last_event_id, last_entry_hash);
    let sig: Signature = signing_key.sign(&pre);
    AuditRecord::Checkpoint {
        run_id: run_id.to_string(),
        created_ts_ms,
        last_event_id,
        last_entry_hash_b64: encode_b64_32(last_entry_hash),
        signature_b64: B64.encode(sig.to_bytes()),
        key_id,
        hash_alg: HASH_ALG.to_string(),
        sig_alg: SIG_ALG.to_string(),
        version: 1,
    }
}

/// Verify an audit JSONL file.
/// - Validates the hash chain across all Event records
/// - Validates signatures on Checkpoint records
pub fn verify_audit_log_file(
    log_path: impl AsRef<Path>,
    pubkey_path: impl AsRef<Path>,
) -> Result<(), String> {
    let vk = load_verify_key_b64(pubkey_path)?;
    let expected_key_id = key_id_from_pubkey(&vk);

    let f = fs::File::open(log_path.as_ref())
        .map_err(|e| format!("failed to open log file {:?}: {e}", log_path.as_ref()))?;
    let reader = BufReader::new(f);

    let mut prev_hash = [0u8; 32];
    let mut last_event_id: u64 = 0;
    let mut run_id_seen: Option<String> = None;

    let mut checkpoints_verified = 0u64;
    let mut events_verified = 0u64;

    for (idx, line_res) in reader.lines().enumerate() {
        let line_no = idx + 1;
        let line = line_res.map_err(|e| format!("line {line_no}: read error: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }

        let rec: AuditRecord =
            serde_json::from_str(&line).map_err(|e| format!("line {line_no}: JSON parse error: {e}"))?;

        match rec {
            AuditRecord::Event { log, integrity } => {
                // Run-id consistency
                if let Some(rid) = &run_id_seen {
                    if &log.run_id != rid {
                        return Err(format!(
                            "line {line_no}: run_id changed ({} -> {})",
                            rid, log.run_id
                        ));
                    }
                } else {
                    run_id_seen = Some(log.run_id.clone());
                }

                // Check prev_hash matches file chain
                let prev_b = decode_b64_32(&integrity.prev_hash_b64)
                    .map_err(|e| format!("line {line_no}: bad prev_hash_b64: {e}"))?;
                if prev_b != prev_hash {
                    return Err(format!(
                        "line {line_no}: prev_hash mismatch (expected {}, got {})",
                        encode_b64_32(&prev_hash),
                        integrity.prev_hash_b64
                    ));
                }

                // Check monotonic event_id (optional but very useful)
                if last_event_id != 0 && log.event_id != last_event_id + 1 {
                    return Err(format!(
                        "line {line_no}: event_id not contiguous (prev {}, got {})",
                        last_event_id, log.event_id
                    ));
                }

                // Recompute entry hash
                let computed = compute_entry_hash(&prev_hash, &log)
                    .map_err(|e| format!("line {line_no}: compute_entry_hash failed: {e}"))?;
                let entry_b = decode_b64_32(&integrity.entry_hash_b64)
                    .map_err(|e| format!("line {line_no}: bad entry_hash_b64: {e}"))?;

                if computed != entry_b {
                    return Err(format!(
                        "line {line_no}: entry_hash mismatch (expected {}, got {})",
                        encode_b64_32(&computed),
                        integrity.entry_hash_b64
                    ));
                }

                // Advance chain tip
                prev_hash = computed;
                last_event_id = log.event_id;
                events_verified += 1;
            }

            AuditRecord::Checkpoint {
                run_id,
                last_event_id: cp_last_event_id,
                last_entry_hash_b64,
                signature_b64,
                key_id,
                hash_alg: _,
                sig_alg: _,
                version: _,
                created_ts_ms: _,
            } => {
                // Bind checkpoint to same run
                if let Some(rid) = &run_id_seen {
                    if &run_id != rid {
                        return Err(format!(
                            "line {line_no}: checkpoint run_id mismatch (expected {}, got {})",
                            rid, run_id
                        ));
                    }
                } else {
                    run_id_seen = Some(run_id.clone());
                }

                // Must match current chain tip
                let cp_hash = decode_b64_32(&last_entry_hash_b64)
                    .map_err(|e| format!("line {line_no}: bad checkpoint last_entry_hash_b64: {e}"))?;
                if cp_hash != prev_hash {
                    return Err(format!(
                        "line {line_no}: checkpoint hash does not match current chain tip"
                    ));
                }

                if cp_last_event_id != last_event_id {
                    return Err(format!(
                        "line {line_no}: checkpoint last_event_id {} does not match stream last_event_id {}",
                        cp_last_event_id, last_event_id
                    ));
                }

                if key_id != expected_key_id {
                    return Err(format!(
                        "line {line_no}: checkpoint key_id mismatch (expected {}, got {})",
                        expected_key_id, key_id
                    ));
                }

                let sig_bytes = B64
                    .decode(signature_b64)
                    .map_err(|e| format!("line {line_no}: bad signature_b64: {e}"))?;
                if sig_bytes.len() != 64 {
                    return Err(format!("line {line_no}: signature length {} != 64", sig_bytes.len()));
                }
                let mut sig64 = [0u8; 64];
                sig64.copy_from_slice(&sig_bytes);
                let sig = Signature::from_bytes(&sig64);

                let pre = checkpoint_preimage(&run_id, cp_last_event_id, &cp_hash);
                vk.verify_strict(&pre, &sig)
                    .map_err(|e| format!("line {line_no}: signature verify failed: {e}"))?;

                checkpoints_verified += 1;
            }
        }
    }

    if events_verified == 0 {
        return Err("no Event records found".to_string());
    }
    if checkpoints_verified == 0 {
        return Err("no Checkpoint records found (did you set checkpoint interval too high?)".to_string());
    }

    Ok(())
}
