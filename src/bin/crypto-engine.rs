// High-performance cryptographic operations for audit logs
// Compile: cargo build --bin crypto-engine --release
// Run: ./target/release/crypto-engine --mode sign --input log.jsonl

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 5 {
        println!("Usage: {} --mode <sign|verify|encrypt> --input <file> [--output <file>]", args[0]);
        return;
    }

    let mode = get_arg_value(&args, "--mode");
    let input_file = get_arg_value(&args, "--input");
    let output_file = get_arg_value(&args, "--output");

    match mode.as_deref() {
        Some("sign") => {
            if let Some(input) = input_file {
                sign_logs(&input, output_file.as_deref());
            } else {
                println!("Error: --input required for sign mode");
            }
        }
        Some("verify") => {
            if let Some(input) = input_file {
                verify_logs(&input);
            } else {
                println!("Error: --input required for verify mode");
            }
        }
        Some("encrypt") => {
            if let Some(input) = input_file {
                encrypt_logs(&input, output_file.as_deref());
            } else {
                println!("Error: --input required for encrypt mode");
            }
        }
        _ => println!("Unknown mode. Use: sign, verify, or encrypt"),
    }
}

fn get_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|pos| args.get(pos + 1).cloned())
}

fn sign_logs(input_path: &str, output_path: Option<&str>) {
    match File::open(input_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut output: Box<dyn Write> = match output_path {
                Some(path) => {
                    match File::create(path) {
                        Ok(f) => Box::new(f),
                        Err(e) => {
                            eprintln!("Failed to create output file: {}", e);
                            Box::new(std::io::stdout())
                        }
                    }
                }
                None => Box::new(std::io::stdout()),
            };

            for (line_num, line) in reader.lines().enumerate() {
                match line {
                    Ok(log_line) if !log_line.trim().is_empty() => {
                        // HMAC-SHA256 signature (simplified - in production use proper HMAC)
                        let signature = format!("{:x}", hash_line(&log_line));
                        writeln!(output, "{}|{}", signature, log_line)
                            .expect("Failed to write output");
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        eprintln!("Error reading line {}: {}", line_num + 1, e);
                    }
                }
            }
            println!("[CRYPTO] Signed {} lines", input_path);
        }
        Err(e) => {
            eprintln!("Failed to open file {}: {}", input_path, e);
        }
    }
}

fn verify_logs(input_path: &str) {
    match File::open(input_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut valid_count = 0;
            let mut invalid_count = 0;

            for (line_num, line) in reader.lines().enumerate() {
                match line {
                    Ok(log_line) if !log_line.trim().is_empty() => {
                        if let Some((signature, content)) = log_line.split_once('|') {
                            let computed = format!("{:x}", hash_line(content));
                            if computed == signature {
                                valid_count += 1;
                            } else {
                                invalid_count += 1;
                                eprintln!("[TAMPER] Line {}: signature mismatch", line_num + 1);
                            }
                        } else {
                            eprintln!("[ERROR] Line {}: invalid format (expected signature|content)", line_num + 1);
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

            println!("[CRYPTO] Verification complete: {} valid, {} invalid", valid_count, invalid_count);
            if invalid_count > 0 {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to open file {}: {}", input_path, e);
            std::process::exit(1);
        }
    }
}

fn encrypt_logs(input_path: &str, output_path: Option<&str>) {
    println!("[CRYPTO] Encryption mode requires additional dependencies (ring or openssl)");
    println!("[CRYPTO] For production, use the TypeScript implementation with Node.js crypto");
    
    // In production, this would use a proper AES-256-GCM implementation
    // For now, we'll just copy the file
    if let Some(output) = output_path {
        match std::fs::copy(input_path, output) {
            Ok(_) => println!("[CRYPTO] File copied (encryption not implemented in Rust binary)"),
            Err(e) => eprintln!("Failed to copy file: {}", e),
        }
    }
}

fn hash_line(line: &str) -> u64 {
    // DJB2 hash algorithm (fast, but not cryptographically secure)
    // For production, use SHA-256 or HMAC-SHA256
    let mut hash: u64 = 5381;
    for c in line.chars() {
        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(c as u64);
    }
    hash
}
