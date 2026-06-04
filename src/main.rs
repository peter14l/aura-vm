use anyhow::Result;
use aura_vm::AuraSandbox;
use clap::Parser;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about = "AuraVM: Hyper-fast MicroVM for AI Agents", long_about = None)]
struct Args {
    /// Path to the .wasm file to execute
    #[arg(short, long)]
    file: String,

    /// Name of the exported function to call
    #[arg(short, long, default_value = "_start")]
    function: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("[AuraVM] Booting Secure MicroVM...");
    let sandbox = AuraSandbox::new()?;
    
    println!("[AuraVM] Loading AI Agent binary: {}", args.file);
    let wasm_bytes = fs::read(&args.file)?;

    println!("[AuraVM] Executing in 10MB isolated sandbox with strict fuel limits...");
    let report = sandbox.execute_agent_code(&wasm_bytes, &args.function)?;
    
    println!("\n[AuraVM] Execution Report:\n{}", serde_json::to_string_pretty(&report)?);
    
    Ok(())
}
