use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;

/// Generate an Ed25519 keypair and write base64-encoded files.
pub fn generate_keypair(out_dir: impl AsRef<Path>) -> Result<(), String> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)
        .map_err(|e| format!("failed to create output dir {:?}: {}", out_dir, e))?;

    // Secure random seed
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let seed_b64 = B64.encode(signing_key.to_bytes());
    let pub_b64 = B64.encode(verifying_key.to_bytes());

    let seed_path = out_dir.join("sentinel_seed.b64");
    let pub_path = out_dir.join("sentinel_pub.b64");

    fs::write(&seed_path, format!("{}\n", seed_b64))
        .map_err(|e| format!("failed to write {:?}: {}", seed_path, e))?;

    fs::write(&pub_path, format!("{}\n", pub_b64))
        .map_err(|e| format!("failed to write {:?}: {}", pub_path, e))?;

    println!("Generated Sentinel signing keypair:");
    println!("  Private key (KEEP SECRET): {:?}", seed_path);
    println!("  Public key  (SHARE):       {:?}", pub_path);

    Ok(())
}
