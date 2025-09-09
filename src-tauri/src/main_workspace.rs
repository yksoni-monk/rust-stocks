use std::process::Command;

fn main() {
    println!("🚀 Stock Analysis System Launcher");
    println!("Starting Tauri application...\n");
    
    // Change to src-tauri directory and run the main application
    let output = Command::new("cargo")
        .args(&["run", "--bin", "rust-stocks-tauri"])
        .current_dir("src-tauri")
        .spawn();
    
    match output {
        Ok(mut child) => {
            // Wait for the child process to finish
            match child.wait() {
                Ok(status) => {
                    if status.success() {
                        println!("✅ Application exited successfully");
                    } else {
                        println!("❌ Application exited with error: {:?}", status);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to wait for application: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to start Tauri application: {}", e);
            eprintln!("💡 Try running from src-tauri directory:");
            eprintln!("   cd src-tauri && cargo run");
        }
    }
}