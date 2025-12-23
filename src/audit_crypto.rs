// src/audit_crypto.rs

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use x25519_dalek::{x25519, PublicKey, X25519_BASEPOINT_BYTES};
use zeroize::Zeroize;

use tokio::io::{AsyncWrite, AsyncWriteExt};

/// ===== Key generation =====
/// We store recipient "private key" as raw 32 bytes (base64).
/// We compute pubkey via X25519(sk, basepoint).
pub fn keygen_recipient(out_dir: impl AsRef<Path>) -> Result<(), String> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)
        .map_err(|e| format!("failed to create {:?}: {}", out_dir, e))?;

    let mut sk = [0u8; 32];
    OsRng.fill_bytes(&mut sk);

    let pk_bytes = x25519(sk, X25519_BASEPOINT_BYTES);
    let pk = PublicKey::from(pk_bytes);

    fs::write(out_dir.join("recipient_priv.b64"), format!("{}\n", B64.encode(sk)))
        .map_err(|e| format!("write recipient_priv.b64: {}", e))?;
    fs::write(
        out_dir.join("recipient_pub.b64"),
        format!("{}\n", B64.encode(pk.as_bytes())),
    )
    .map_err(|e| format!("write recipient_pub.b64: {}", e))?;

    println!("Generated recipient encryption keys (X25519)");
    println!("  Private (KEEP SECRET): {:?}", out_dir.join("recipient_priv.b64"));
    println!("  Public  (DISTRIBUTE):  {:?}", out_dir.join("recipient_pub.b64"));
    Ok(())
}

/// ===== Internal helpers =====

fn read_b64_32(path: &Path) -> Result<[u8; 32], String> {
    let s = fs::read_to_string(path).map_err(|e| format!("read {:?}: {}", path, e))?;
    let bytes = B64
        .decode(s.trim())
        .map_err(|e| format!("base64 decode {:?}: {}", path, e))?;
    if bytes.len() != 32 {
        return Err(format!(
            "expected 32 bytes in {:?}, got {}",
            path,
            bytes.len()
        ));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn key_id(pk: &[u8; 32]) -> String {
    let h = Sha256::digest(pk);
    hex::encode(&h[..6])
}

#[derive(Clone)]
struct DataKey([u8; 32]);

impl Drop for DataKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl DataKey {
    fn random() -> Self {
        let mut dk = [0u8; 32];
        OsRng.fill_bytes(&mut dk);
        Self(dk)
    }
}

/// ===== Data structures =====

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyEnvelope {
    pub record_type: String, // "KeyEnvelope"
    pub version: u32,

    pub run_id: String,

    pub recipient_key_id: String,

    pub ephemeral_pubkey_b64: String,

    pub wrap_nonce_b64: String,
    pub wrapped_dek_b64: String,

    pub kex_alg: String,
    pub kdf_alg: String,
    pub aead_alg: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedRecord {
    record_type: String, // "Encrypted"
    version: u32,
    run_id: String,
    inner_type: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

/// ===== Envelope logic =====

fn build_envelope(run_id: &str, recipient_pub: &PublicKey, dek: &DataKey) -> KeyEnvelope {
    // Generate ephemeral secret bytes + derive ephemeral pubkey.
    let mut eph_sk = [0u8; 32];
    OsRng.fill_bytes(&mut eph_sk);

    let eph_pk_bytes = x25519(eph_sk, X25519_BASEPOINT_BYTES);
    let eph_pk = PublicKey::from(eph_pk_bytes);

    // X25519 shared secret: x25519(eph_sk, recipient_pub)
    let shared = x25519(eph_sk, *recipient_pub.as_bytes());
    // HKDF over shared secret
    let hk = Hkdf::<Sha256>::new(None, &shared);

    let mut wrap_key = [0u8; 32];
    hk.expand(b"sentinel/dek-wrap/v1", &mut wrap_key)
        .expect("hkdf expand");

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&wrap_key));

    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);

    let wrapped = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &dek.0,
                aad: run_id.as_bytes(),
            },
        )
        .expect("wrap encrypt");

    KeyEnvelope {
        record_type: "KeyEnvelope".into(),
        version: 1,
        run_id: run_id.into(),
        recipient_key_id: key_id(recipient_pub.as_bytes()),
        ephemeral_pubkey_b64: B64.encode(eph_pk.as_bytes()),
        wrap_nonce_b64: B64.encode(nonce),
        wrapped_dek_b64: B64.encode(wrapped),
        kex_alg: "x25519".into(),
        kdf_alg: "hkdf-sha256".into(),
        aead_alg: "chacha20poly1305".into(),
    }
}

fn unwrap_envelope(env: &KeyEnvelope, recipient_sk: &[u8; 32]) -> Result<DataKey, String> {
    let eph_pk_bytes = B64
        .decode(env.ephemeral_pubkey_b64.trim())
        .map_err(|e| format!("decode ephemeral_pubkey_b64: {}", e))?;
    if eph_pk_bytes.len() != 32 {
        return Err("bad ephemeral pubkey length".to_string());
    }
    let mut eph_pk_arr = [0u8; 32];
    eph_pk_arr.copy_from_slice(&eph_pk_bytes);

    // shared = x25519(recipient_sk, eph_pk)
    let shared = x25519(*recipient_sk, eph_pk_arr);
    let hk = Hkdf::<Sha256>::new(None, &shared);

    let mut wrap_key = [0u8; 32];
    hk.expand(b"sentinel/dek-wrap/v1", &mut wrap_key)
        .map_err(|_| "hkdf expand failed".to_string())?;

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&wrap_key));

    let nonce_bytes = B64
        .decode(env.wrap_nonce_b64.trim())
        .map_err(|e| format!("decode wrap_nonce_b64: {}", e))?;
    if nonce_bytes.len() != 12 {
        return Err("bad wrap nonce length".to_string());
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes);

    let wrapped = B64
        .decode(env.wrapped_dek_b64.trim())
        .map_err(|e| format!("decode wrapped_dek_b64: {}", e))?;

    let dek_bytes = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &wrapped,
                aad: env.run_id.as_bytes(),
            },
        )
        .map_err(|_| "failed to unwrap DEK (bad key or tampered envelope)".to_string())?;

    if dek_bytes.len() != 32 {
        return Err("bad DEK length after unwrap".to_string());
    }
    let mut dk = [0u8; 32];
    dk.copy_from_slice(&dek_bytes);
    Ok(DataKey(dk))
}

/// ===== AuditSink (PLAINTEXT or ENCRYPTED) =====

pub enum AuditSink<'a, W: AsyncWrite + Unpin> {
    Plain { out: &'a mut W },
    Encrypted {
        out: &'a mut W,
        run_id: String,
        dek: DataKey,
    },
}

impl<'a, W: AsyncWrite + Unpin> AuditSink<'a, W> {
    pub async fn new(
        out: &'a mut W,
        run_id: &str,
        recipient_pub_path: Option<&str>,
    ) -> Result<Self, String> {
        if let Some(path) = recipient_pub_path {
            let pub_bytes = read_b64_32(Path::new(path))?;
            let recipient_pub = PublicKey::from(pub_bytes);

            let dek = DataKey::random();
            let env = build_envelope(run_id, &recipient_pub, &dek);

            let line = serde_json::to_string(&env).map_err(|e| format!("serialize env: {}", e))?;
            out.write_all(format!("{}\n", line).as_bytes())
                .await
                .map_err(|e| format!("write KeyEnvelope: {}", e))?;

            Ok(Self::Encrypted {
                out,
                run_id: run_id.into(),
                dek,
            })
        } else {
            Ok(Self::Plain { out })
        }
    }

    pub async fn write_record(&mut self, inner: &str, json: &str) -> Result<(), String> {
        match self {
            Self::Plain { out } => {
                out.write_all(format!("{}\n", json).as_bytes())
                    .await
                    .map_err(|e| format!("write plaintext: {}", e))?;
            }
            Self::Encrypted { out, run_id, dek } => {
                let cipher = ChaCha20Poly1305::new(Key::from_slice(&dek.0));
                let mut nonce = [0u8; 12];
                OsRng.fill_bytes(&mut nonce);

                let aad = format!("{}|{}", run_id, inner);
                let ct = cipher
                    .encrypt(
                        Nonce::from_slice(&nonce),
                        Payload {
                            msg: json.as_bytes(),
                            aad: aad.as_bytes(),
                        },
                    )
                    .map_err(|_| "encrypt failed".to_string())?;

                let rec = EncryptedRecord {
                    record_type: "Encrypted".into(),
                    version: 1,
                    run_id: run_id.clone(),
                    inner_type: inner.into(),
                    nonce_b64: B64.encode(nonce),
                    ciphertext_b64: B64.encode(ct),
                };

                let line =
                    serde_json::to_string(&rec).map_err(|e| format!("serialize enc: {}", e))?;
                out.write_all(format!("{}\n", line).as_bytes())
                    .await
                    .map_err(|e| format!("write encrypted: {}", e))?;
            }
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), String> {
        match self {
            Self::Plain { out } => out.flush().await.map_err(|e| format!("flush: {}", e))?,
            Self::Encrypted { out, .. } => out.flush().await.map_err(|e| format!("flush: {}", e))?,
        }
        Ok(())
    }
}

/// If the log starts with KeyEnvelope, decrypt it into a plaintext temp file and return that path.
/// If it does not, return the original log path.
///
/// we use NamedTempFile::keep() so the returned PathBuf actually exists after returning.
pub fn maybe_decrypt_to_temp_plaintext(
    log_path: &str,
    recipient_privkey_b64_path: Option<&str>,
) -> Result<PathBuf, String> {
    // Peek first line
    let file = File::open(log_path).map_err(|e| format!("open audit log: {}", e))?;
    let mut reader = BufReader::new(file);

    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .map_err(|e| format!("read first line: {}", e))?;

    if first_line.trim().is_empty() {
        return Err("audit log is empty".to_string());
    }

    // If not a KeyEnvelope -> plaintext log
    let env_parse = serde_json::from_str::<KeyEnvelope>(first_line.trim());
    if env_parse.is_err() {
        return Ok(PathBuf::from(log_path));
    }
    let env = env_parse.unwrap();
    if env.record_type != "KeyEnvelope" {
        // Treat as plaintext if some other record got put first
        return Ok(PathBuf::from(log_path));
    }

    // Encrypted log -> need recipient priv key
    let priv_path = recipient_privkey_b64_path
        .ok_or("encrypted audit log requires recipient private key for verification")?;
    let recipient_sk = read_b64_32(Path::new(priv_path))?;

    // Derive DEK from envelope
    let dek = unwrap_envelope(&env, &recipient_sk)?;

    // Re-open and stream decrypt the rest (starting AFTER first line)
    let file = File::open(log_path).map_err(|e| format!("re-open audit log: {}", e))?;
    let reader = BufReader::new(file);

    let mut tmp =
        NamedTempFile::new().map_err(|e| format!("create temp file: {}", e))?;

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&dek.0));
    let mut saw_first = false;

    for line_res in reader.lines() {
        let line = line_res.map_err(|e| format!("read line: {}", e))?;
        let s = line.trim();
        if s.is_empty() {
            continue;
        }

        // Skip the first line (KeyEnvelope)
        if !saw_first {
            saw_first = true;
            continue;
        }

        let rec: EncryptedRecord =
            serde_json::from_str(s).map_err(|e| format!("parse EncryptedRecord: {}", e))?;
        if rec.record_type != "Encrypted" {
            return Err(format!("unexpected record_type {}", rec.record_type));
        }
        if rec.run_id != env.run_id {
            return Err("run_id mismatch (possible splicing)".to_string());
        }

        let nonce_bytes = B64
            .decode(rec.nonce_b64.trim())
            .map_err(|e| format!("decode nonce: {}", e))?;
        if nonce_bytes.len() != 12 {
            return Err("bad nonce length".to_string());
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&nonce_bytes);

        let ct = B64
            .decode(rec.ciphertext_b64.trim())
            .map_err(|e| format!("decode ciphertext: {}", e))?;

        let aad = format!("{}|{}", env.run_id, rec.inner_type);

        let pt = cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &ct,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| "decrypt failed (bad key or tampered ciphertext)".to_string())?;

        let pt_str =
            String::from_utf8(pt).map_err(|_| "decrypted payload not utf8".to_string())?;

        writeln!(tmp, "{}", pt_str).map_err(|e| format!("write decrypted: {}", e))?;
    }

    // Persist the tempfile so returning PathBuf is valid
    let (_file, path) = tmp
        .keep()
        .map_err(|e| format!("persist temp file: {}", e))?;

    Ok(path)
}
