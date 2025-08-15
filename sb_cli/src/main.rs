use clap::Parser;
use sb_harness::{RunResult, SbHarnessFactory};
use std::{collections::BTreeMap, fs};

/// Simple CLI to run a WASM module with os_print import and call program_run
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the WASM file
    #[arg(long)]
    wasm: String,
}

fn main() {
    let args = Args::parse();
    let wasm_bytes =
        fs::read(&args.wasm).unwrap_or_else(|e| panic!("Failed to read wasm file: {}", e));

    let harness_factory = SbHarnessFactory::new().expect("Failed to create harness factory");

    println!("Creating program harnesses for {}", args.wasm);
    let mut harnesses: Vec<_> = (0..10)
        .map(|i| {
            let proc_name = format!("hello_{i}");
            let proc_blurb = format!("Hello from {proc_name}!");
            Some(
                harness_factory
                    .create(&wasm_bytes, i, proc_name, proc_blurb, 1000)
                    .expect("Failed to create program harness"),
            )
        })
        .collect();

    println!("Created {} program harnesses. Running!", harnesses.len());
    let mut results: BTreeMap<u32, u32> = BTreeMap::new();
    let mut epoch = 0;
    while results.len() < harnesses.len() {
        epoch += 1;
        println!("Epoch {epoch}");
        for (i, harness_slot) in harnesses.iter_mut().enumerate() {
            if let Some(mut harness) = harness_slot.take() {
                match harness.step() {
                    Ok(RunResult::Complete(result)) => {
                        results.insert(i as u32, result);
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error running process {}: {}", i, e);
                    }
                    _ => {}
                }
                harness_slot.replace(harness);
            }
        }
    }

    println!("Process run result: {:?}", results);
    println!("Program run completed successfully.");
}
