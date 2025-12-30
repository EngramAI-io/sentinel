use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs::File;
use std::io::{BufRead, BufReader};
use x25519_dalek::{x25519, PublicKey};

#[derive(Debug, Deserialize)]
struct KeyEnvelope {
    record_type: String,
    version: u32,
    run_id: String,
    recipient_key_id: String,
    ephemeral_pubkey_b64: String,
    wrap_nonce_b64: String,
    wrapped_dek_b64: String,
    kex_alg: String,
    kdf_alg: String,
    aead_alg: String,
}

#[derive(Debug, Deserialize)]
struct EncryptedRecord {
    record_type: String,
    version: u32,
    run_id: String,
    inner_type: String,
    nonce_b64: String,
    ciphertext_b64: String,
}

fn read_privkey_b64(path: &str) -> Result<[u8; 32]> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read private key file")?;
    let bytes = B64.decode(content.trim())
        .context("Failed to decode base64 private key")?;
    
    if bytes.len() != 32 {
        anyhow::bail!("Private key must be 32 bytes, got {}", bytes.len());
    }
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn unwrap_dek(envelope: &KeyEnvelope, recipient_sk: &[u8; 32]) -> Result<[u8; 32]> {
    // Decode ephemeral public key from envelope
    let eph_pk_bytes = B64.decode(envelope.ephemeral_pubkey_b64.trim())
        .context("Failed to decode ephemeral pubkey")?;
    
    if eph_pk_bytes.len() != 32 {
        anyhow::bail!("Ephemeral pubkey must be 32 bytes");
    }
    
    let mut eph_pk_arr = [0u8; 32];
    eph_pk_arr.copy_from_slice(&eph_pk_bytes);
    
    // Compute shared secret: X25519(recipient_sk, ephemeral_pk)
    let shared_secret = x25519(*recipient_sk, eph_pk_arr);
    
    // Derive wrapping key using HKDF
    let hk = Hkdf::<Sha256>::new(None, &shared_secret);
    let mut wrap_key = [0u8; 32];
    hk.expand(b"sentinel/dek-wrap/v1", &mut wrap_key)
        .context("HKDF expansion failed")?;
    
    // Prepare cipher and nonce for unwrapping
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&wrap_key));
    
    let nonce_bytes = B64.decode(envelope.wrap_nonce_b64.trim())
        .context("Failed to decode wrap nonce")?;
    if nonce_bytes.len() != 12 {
        anyhow::bail!("Wrap nonce must be 12 bytes");
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes);
    
    // Decrypt the wrapped DEK
    let wrapped_dek = B64.decode(envelope.wrapped_dek_b64.trim())
        .context("Failed to decode wrapped DEK")?;
    
    let dek_bytes = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &wrapped_dek,
                aad: envelope.run_id.as_bytes(),
            },
        )
        .context("Failed to unwrap DEK - wrong key or tampered envelope")?;
    
    if dek_bytes.len() != 32 {
        anyhow::bail!("Unwrapped DEK must be 32 bytes");
    }
    
    let mut dek = [0u8; 32];
    dek.copy_from_slice(&dek_bytes);
    Ok(dek)
}

fn decrypt_record(
    encrypted: &EncryptedRecord,
    dek: &[u8; 32],
    run_id: &str,
) -> Result<String> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(dek));
    
    // Decode nonce
    let nonce_bytes = B64.decode(encrypted.nonce_b64.trim())
        .context("Failed to decode nonce")?;
    if nonce_bytes.len() != 12 {
        anyhow::bail!("Nonce must be 12 bytes");
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes);
    
    // Decode ciphertext
    let ciphertext = B64.decode(encrypted.ciphertext_b64.trim())
        .context("Failed to decode ciphertext")?;
    
    // Construct AAD (must match encryption)
    let aad = format!("{}|{}", run_id, encrypted.inner_type);
    
    // Decrypt
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .context("Decryption failed - wrong key or tampered ciphertext")?;
    
    String::from_utf8(plaintext)
        .context("Decrypted plaintext is not valid UTF-8")
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <audit_log.jsonl> <recipient_priv.b64>", args[0]);
        std::process::exit(1);
    }
    
    let log_path = &args[1];
    let privkey_path = &args[2];
    
    // Load recipient private key
    let recipient_sk = read_privkey_b64(privkey_path)?;
    
    // Open audit log
    let file = File::open(log_path)
        .context("Failed to open audit log")?;
    let reader = BufReader::new(file);
    
    let mut dek: Option<[u8; 32]> = None;
    let mut run_id: Option<String> = None;
    let mut line_num = 0;
    
    for line_result in reader.lines() {
        line_num += 1;
        let line = line_result
            .context(format!("Failed to read line {}", line_num))?;
        
        if line.trim().is_empty() {
            continue;
        }
        
        // Try to parse as KeyEnvelope first
        if let Ok(envelope) = serde_json::from_str::<KeyEnvelope>(&line) {
            if envelope.record_type == "KeyEnvelope" {
                eprintln!("🔑 Found KeyEnvelope at line {}", line_num);
                eprintln!("   Run ID: {}", envelope.run_id);
                eprintln!("   Key ID: {}", envelope.recipient_key_id);
                
                // Unwrap the DEK
                let unwrapped_dek = unwrap_dek(&envelope, &recipient_sk)
                    .context(format!("Line {}: Failed to unwrap DEK", line_num))?;
                
                dek = Some(unwrapped_dek);
                run_id = Some(envelope.run_id.clone());
                
                eprintln!("   ✓ DEK unwrapped successfully\n");
                continue;
            }
        }
        
        // Try to parse as EncryptedRecord
        if let Ok(encrypted) = serde_json::from_str::<EncryptedRecord>(&line) {
            if encrypted.record_type == "Encrypted" {
                let current_dek = dek
                    .context(format!("Line {}: Found encrypted record before KeyEnvelope", line_num))?;
                let current_run_id = run_id.as_ref()
                    .context(format!("Line {}: No run_id established", line_num))?;
                
                // Verify run_id consistency
                if encrypted.run_id != *current_run_id {
                    anyhow::bail!(
                        "Line {}: run_id mismatch (expected {}, got {})",
                        line_num,
                        current_run_id,
                        encrypted.run_id
                    );
                }
                
                // Decrypt and print
                let plaintext = decrypt_record(&encrypted, &current_dek, current_run_id)
                    .context(format!("Line {}: Failed to decrypt record", line_num))?;
                
                println!("{}", plaintext);
                continue;
            }
        }
        
        // If not encrypted, assume it's plaintext (Event or Checkpoint)
        println!("{}", line);
    }
    
    eprintln!("\n✓ Decryption complete ({} lines processed)", line_num);
    
    Ok(())
}