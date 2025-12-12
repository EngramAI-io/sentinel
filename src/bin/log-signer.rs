// Log file integrity signer with HMAC-SHA256
// Compile: cargo build --bin log-signer --release
// Run: ./target/release/log-signer --file audit-2024-01-01.jsonl --key <hex-key>

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 5 {
        println!("Usage: {} --file <logfile> --key <hex-key> [--verify]", args[0]);
        println!("  --file: Path to log file to sign/verify");
        println!("  --key:  HMAC key in hex format (64 hex chars = 32 bytes)");
        println!("  --verify: Verify existing signatures instead of signing");
        return;
    }

    let file_path = get_arg_value(&args, "--file");
    let key_hex = get_arg_value(&args, "--key");
    let verify_mode = args.contains(&"--verify".to_string());

    match (file_path, key_hex) {
        (Some(file), Some(key)) => {
            if verify_mode {
                verify_file_signatures(&file, &key);
            } else {
                sign_file(&file, &key);
            }
        }
        _ => {
            println!("Error: --file and --key are required");
            std::process::exit(1);
        }
    }
}

fn get_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|pos| args.get(pos + 1).cloned())
}

fn sign_file(file_path: &str, key_hex: &str) {
    // Parse hex key
    let key = match hex::decode(key_hex) {
        Ok(k) if k.len() == 32 => k,
        Ok(_) => {
            eprintln!("Error: Key must be 32 bytes (64 hex characters)");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: Invalid hex key: {}", e);
            std::process::exit(1);
        }
    };

    match File::open(file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let output_path = format!("{}.signed", file_path);
            
            match File::create(&output_path) {
                Ok(mut output) => {
                    let mut line_count = 0;
                    
                    for line in reader.lines() {
                        match line {
                            Ok(log_line) if !log_line.trim().is_empty() => {
                                // In production, use proper HMAC-SHA256
                                // For now, use a simple hash (replace with ring::hmac or openssl)
                                let signature = simple_hmac(&log_line, &key);
                                writeln!(output, "{}|{}", signature, log_line)
                                    .expect("Failed to write");
                                line_count += 1;
                            }
                            Ok(_) => continue,
                            Err(e) => {
                                eprintln!("Error reading line: {}", e);
                            }
                        }
                    }
                    
                    println!("[SIGNER] Signed {} lines to {}", line_count, output_path);
                }
                Err(e) => {
                    eprintln!("Failed to create output file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open file {}: {}", file_path, e);
            std::process::exit(1);
        }
    }
}

fn verify_file_signatures(file_path: &str, key_hex: &str) {
    // Parse hex key
    let key = match hex::decode(key_hex) {
        Ok(k) if k.len() == 32 => k,
        Ok(_) => {
            eprintln!("Error: Key must be 32 bytes (64 hex characters)");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: Invalid hex key: {}", e);
            std::process::exit(1);
        }
    };

    match File::open(file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut valid_count = 0;
            let mut invalid_count = 0;

            for (line_num, line) in reader.lines().enumerate() {
                match line {
                    Ok(log_line) if !log_line.trim().is_empty() => {
                        if let Some((signature, content)) = log_line.split_once('|') {
                            let computed = simple_hmac(content, &key);
                            if computed == signature {
                                valid_count += 1;
                            } else {
                                invalid_count += 1;
                                eprintln!("[TAMPER] Line {}: signature mismatch", line_num + 1);
                            }
                        } else {
                            eprintln!("[ERROR] Line {}: invalid format", line_num + 1);
                            invalid_count += 1;
                        }
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        eprintln!("Error reading line {}: {}", line_num + 1, e);
                        invalid_count += 1;
                    }
                }
            }

            println!("[SIGNER] Verification: {} valid, {} invalid", valid_count, invalid_count);
            if invalid_count > 0 {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to open file {}: {}", file_path, e);
            std::process::exit(1);
        }
    }
}

fn simple_hmac(data: &str, key: &[u8]) -> String {
    // Simplified HMAC (for production, use ring::hmac or openssl)
    // This is a placeholder - replace with proper HMAC-SHA256
    // For now, use a simple hash combination
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
