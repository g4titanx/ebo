mod evm;
mod obfuscator;

use crate::obfuscator::Obfuscator;
use clap::{Parser, Subcommand, ValueEnum};
use log::{debug, info};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ebo", about = "EVM Bytecode Obfuscator with Chaotic Shuffle")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Obfuscate EVM bytecode
    Obfuscate {
        /// Input bytecode file path
        #[arg(long, required = true)]
        file: PathBuf,
        /// Random seed for obfuscation
        #[arg(long, default_value = "42")]
        seed: u64,
        /// Verbosity level
        #[arg(long, value_enum, default_value_t = Verbosity::Normal)]
        verbosity: Verbosity,
    },
}
#[derive(ValueEnum, Clone, PartialEq)]
enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Obfuscate {
            file,
            seed,
            verbosity,
        } => {
            match verbosity {
                Verbosity::Quiet => std::env::set_var("RUST_LOG", "error"),
                Verbosity::Normal => std::env::set_var("RUST_LOG", "info"),
                Verbosity::Verbose => std::env::set_var("RUST_LOG", "debug"),
            }

            info!("Starting EVM Bytecode Obfuscator");

            info!("Reading bytecode from file: {:?}", file);
            let bytecode = std::fs::read(&file)?;

            let mut obfuscator = Obfuscator::new(&bytecode, seed);
            info!("Obfuscating bytecode...");
            let obfuscated = obfuscator.obfuscate();

            if verbosity == Verbosity::Verbose {
                debug!("Original bytecode: {}", hex::encode(&bytecode));
                debug!("Obfuscated bytecode: {}", hex::encode(&obfuscated));
                debug!(
                    "Bytecode length increase: {}%",
                    ((obfuscated.len() as f64 / bytecode.len() as f64) - 1.0) * 100.0
                );
            } else {
                info!(
                    "Obfuscation complete. Output length: {} bytes",
                    obfuscated.len()
                );
            }

            let output_path = "obfuscated.bin";
            std::fs::write(output_path, &obfuscated)?;
            info!("Obfuscated bytecode saved to {}", output_path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::evm::{compute_cfg_complexity, parse_bytecode, Opcode};
    use crate::obfuscator::Obfuscator;
    use proptest::prelude::*;
    use std::fs;

    // Helper to count unique opcodes for readability metric
    fn count_unique_opcodes(bytecode: &[u8]) -> usize {
        let mut unique = std::collections::HashSet::new();
        for &b in bytecode {
            unique.insert(b);
        }
        unique.len()
    }

    // Simplified Halstead's Effort proxy (operators + operands)
    fn halstead_effort_proxy(bytecode: &[u8]) -> f64 {
        let n1 = count_unique_opcodes(bytecode) as f64; // Unique operators
        let n2 = bytecode.len() as f64; // Total operands
        let effort = n1 * n2 * n2.log2(); // Simplified effort
        effort
    }

    #[test]
    fn test_obfuscate_add() {
        let bytecode = vec![0x01]; // ADD
        let mut obfuscator = Obfuscator::new(&bytecode, 42);
        let obfuscated = obfuscator.obfuscate();
        assert!(!obfuscated.is_empty());
        assert!(obfuscated == vec![0x01] || obfuscated == vec![0x60, 0x01, 0x01, 0x60, 0x01, 0x01]);
    }

    #[test]
    fn test_obfuscate_jumpy_false_branch() {
        let bytecode = vec![0x57]; // JUMPI
        let mut obfuscator = Obfuscator::new(&bytecode, 42);
        let obfuscated = obfuscator.obfuscate();
        assert!(obfuscated.len() >= 1);
        assert_eq!(obfuscated[0], 0x57);
        if obfuscated.len() > 1 {
            assert_eq!(obfuscated[1], 0x5B); // JUMPDEST
        }
    }

    #[test]
    fn test_obfuscate_stop_dead_code() {
        let bytecode = vec![0x00]; // STOP
        let mut obfuscator = Obfuscator::new(&bytecode, 42);
        let obfuscated = obfuscator.obfuscate();
        assert!(obfuscated.len() >= 1);
        assert_eq!(obfuscated[0], 0x00);
    }

    #[test]
    fn test_chaotic_shuffle_preserves_control_flow() {
        let bytecode = vec![0x01, 0x01, 0x57, 0x00]; // ADD, ADD, JUMPI, STOP
        let mut obfuscator = Obfuscator::new(&bytecode, 42);
        let obfuscated = obfuscator.obfuscate();
        let blocks = parse_bytecode(&obfuscated);
        assert!(blocks.iter().any(|b| b.opcodes.contains(&Opcode::JUMPI)));
        assert!(blocks.iter().any(|b| b.opcodes.contains(&Opcode::STOP)));
    }

    #[test]
    fn test_cfg_complexity_increase() {
        let bytecode = vec![0x01, 0x57, 0x00]; // ADD, JUMPI, STOP
        let original_blocks = parse_bytecode(&bytecode);
        let original_complexity = compute_cfg_complexity(&original_blocks);
        let mut obfuscator = Obfuscator::new(&bytecode, 42);
        let obfuscated = obfuscator.obfuscate();
        let obfuscated_blocks = parse_bytecode(&obfuscated);
        let obfuscated_complexity = compute_cfg_complexity(&obfuscated_blocks);
        assert!(obfuscated_complexity >= original_complexity);
    }

    #[test]
    fn test_incrementer_obfuscation() {
        // Try reading full bytecode, fall back to snippet
        let bytecode = fs::read("examples/incrementer.bin").unwrap_or_else(|_| {
            vec![
                0x60, 0x01, 0x54, // PUSH1 1, SLOAD
                0x60, 0x01, 0x01, // PUSH1 1, ADD
                0x55, // SSTORE
                0x60, 0x00, 0x52, // PUSH1 0, MSTORE
                0x60, 0x20, 0x60, 0x00, 0xF3, // PUSH1 32, PUSH1 0, RETURN
            ]
        });
        let original_blocks = parse_bytecode(&bytecode);
        let original_complexity = compute_cfg_complexity(&original_blocks);
        let original_unique_opcodes = count_unique_opcodes(&bytecode);
        let original_effort = halstead_effort_proxy(&bytecode);

        let mut obfuscator = Obfuscator::new(&bytecode, 42);
        let obfuscated = obfuscator.obfuscate();
        let obfuscated_blocks = parse_bytecode(&obfuscated);
        let obfuscated_complexity = compute_cfg_complexity(&obfuscated_blocks);
        let obfuscated_unique_opcodes = count_unique_opcodes(&obfuscated);
        let obfuscated_effort = halstead_effort_proxy(&obfuscated);

        // Verify functionality
        assert!(obfuscated.iter().any(|&b| b == 0x54)); // SLOAD
        assert!(obfuscated.iter().any(|&b| b == 0x55)); // SSTORE
        assert!(obfuscated.iter().any(|&b| b == 0xF3)); // RETURN

        // Verify reverse engineering resistance
        assert!(obfuscated_complexity >= original_complexity); // More JUMPI
        assert!(obfuscated_unique_opcodes >= original_unique_opcodes); // More opcode variety
        assert!(obfuscated_effort > original_effort); // Higher analysis effort
    }

    proptest! {
        #[test]
        fn fuzz_obfuscation_does_not_crash(bytecode in prop::collection::vec(0u8..=255u8, 0..100), seed in 0u64..1000u64) {
            let mut obfuscator = Obfuscator::new(&bytecode, seed);
            let _obfuscated = obfuscator.obfuscate();
        }
    }
}
