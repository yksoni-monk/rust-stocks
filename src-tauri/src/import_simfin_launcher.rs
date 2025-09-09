use std::env;
use std::process::Command;

fn main() {
    println!("📊 SimFin Data Import Launcher");
    println!("Delegating to the Tauri backend import tool...\n");
    
    // Get command line arguments (skip the first one which is the program name)
    let args: Vec<String> = env::args().skip(1).collect();
    
    // If no arguments provided, show help
    if args.is_empty() {
        println!("Usage: cargo run --bin import-simfin -- [OPTIONS]");
        println!("Example: cargo run --bin import-simfin -- --prices ~/simfin_data/us-shareprices-daily.csv --income ~/simfin_data/us-income-quarterly.csv");
        println!("\nRunning with --help to show full options...\n");
        
        // Run with --help flag
        let output = Command::new("cargo")
            .args(&["run", "--bin", "import_simfin", "--", "--help"])
            .current_dir("src-tauri")
            .spawn();
        
        match output {
            Ok(mut child) => {
                let _ = child.wait();
            }
            Err(e) => {
                eprintln!("❌ Failed to show help: {}", e);
            }
        }
        return;
    }
    
    // Build the command with arguments
    let mut cmd_args = vec!["run", "--bin", "import_simfin", "--"];
    for arg in &args {
        cmd_args.push(arg);
    }
    
    println!("🔧 Running: cargo {}", cmd_args.join(" "));
    println!("📁 Working directory: src-tauri/\n");
    
    // Change to src-tauri directory and run the import tool
    let output = Command::new("cargo")
        .args(&cmd_args)
        .current_dir("src-tauri")
        .spawn();
    
    match output {
        Ok(mut child) => {
            // Wait for the child process to finish
            match child.wait() {
                Ok(status) => {
                    if status.success() {
                        println!("\n✅ SimFin import completed successfully");
                    } else {
                        println!("\n❌ SimFin import failed with status: {:?}", status);
                        println!("💡 Check the error messages above for details");
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to wait for import process: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to start SimFin import tool: {}", e);
            eprintln!("💡 Try running directly from src-tauri directory:");
            eprintln!("   cd src-tauri && cargo run --bin import_simfin -- --help");
        }
    }
}