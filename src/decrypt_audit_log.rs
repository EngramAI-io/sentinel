use std::{fs::File, io::{BufRead, BufReader}};
use base64::{engine::general_purpose, Engine};
use serde_json::Value;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use x25519_dalek::{StaticSecret, PublicKey};

fn main() -> anyhow::Result<()> {
    let audit = File::open("sentinel_audit.jsonl")?;
    let privkey_b64 = std::fs::read_to_string("recipient_priv.b64")?;
    let privkey = StaticSecret::from(
        general_purpose::STANDARD.decode(privkey_b64.trim())?
    );

    let reader = BufReader::new(audit);
    let mut shared_key: Option<[u8; 32]> = None;

    for line in reader.lines() {
        let rec: Value = serde_json::from_str(&line?)?;

        if rec["type"] == "KeyEnvelope" {
            let sender_pub = PublicKey::from(
                general_purpose::STANDARD.decode(
                    rec["sender_pubkey"].as_str().unwrap()
                )?
                .as_slice()
                .try_into()
                .unwrap()
            );
            shared_key = Some(privkey.diffie_hellman(&sender_pub).to_bytes());
            println!("{}", rec);
            continue;
        }

        if let Some(sk) = shared_key {
            if rec.get("ciphertext").is_some() {
                let nonce = general_purpose::STANDARD.decode(rec["nonce"].as_str().unwrap())?;
                let ct = general_purpose::STANDARD.decode(rec["ciphertext"].as_str().unwrap())?;
                let aad = rec["aad"].as_str().unwrap().as_bytes();

                let cipher = ChaCha20Poly1305::new_from_slice(&sk)?;
                let pt = cipher.decrypt(&nonce.into(), ct.as_ref(), aad)?;
                println!("{}", String::from_utf8(pt)?);
                continue;
            }
        }

        println!("{}", rec);
    }

    Ok(())
}
