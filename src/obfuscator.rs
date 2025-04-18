use crate::evm::{parse_bytecode, Opcode};
use log::debug;
use rand::{rngs::StdRng, Rng, SeedableRng};
use sha2::{Digest, Sha256};

pub struct Obfuscator {
    bytecode: Vec<u8>,
    rng: StdRng,
    chaotic_seed: f64,
}

impl Obfuscator {
    pub fn new(bytecode: &[u8], seed: u64) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(seed.to_le_bytes());
        let hash = hasher.finalize();
        let chaotic_seed = f64::from_le_bytes(hash[0..8].try_into().unwrap()) / u64::MAX as f64;

        Obfuscator {
            bytecode: bytecode.to_vec(),
            rng: StdRng::seed_from_u64(seed),
            chaotic_seed,
        }
    }

    fn chaotic_map(&mut self, x: f64) -> f64 {
        let mu = 3.9;
        let p = 0.4;
        if x < p {
            (x.cos() * mu * x.cos()).sin().abs() % 1.0
        } else {
            (1.0 - x).sin() % 1.0
        }
    }

    pub fn obfuscate(&mut self) -> Vec<u8> {
        let blocks = parse_bytecode(&self.bytecode);
        let mut new_bytecode = Vec::new();
        let mut chaotic_val = self.chaotic_seed;

        for block in blocks {
            let mut block_bytes = Vec::new();
            let mut opcodes: Vec<Opcode> = block.opcodes;

            // Chaotic shuffle within block (avoid shuffling jump-related opcodes)
            if self.rng.gen_bool(0.3) {
                chaotic_val = self.chaotic_map(chaotic_val);
                let shuffle_count = (chaotic_val * opcodes.len() as f64) as usize;
                let safe_opcodes: Vec<_> = opcodes
                    .iter()
                    .enumerate()
                    .filter(|(_, op)| !matches!(op, Opcode::JUMPI | Opcode::JUMPDEST))
                    .collect();
                let mut indices: Vec<usize> = safe_opcodes.iter().map(|&(i, _)| i).collect();
                for _ in 0..shuffle_count {
                    if indices.len() > 1 {
                        let i = self.rng.gen_range(0..indices.len());
                        let j = self.rng.gen_range(0..indices.len());
                        indices.swap(i, j);
                    }
                }
                let mut new_opcodes = opcodes.clone();
                for (new_idx, &old_idx) in indices.iter().enumerate() {
                    if let Some((_, op)) = safe_opcodes.get(new_idx) {
                        new_opcodes[old_idx] = (*op).clone();
                    }
                }
                opcodes = new_opcodes;
            }

            // Apply obfuscation techniques
            for op in opcodes {
                match op {
                    Opcode::ADD => {
                        if self.rng.gen_bool(0.5) {
                            block_bytes.extend_from_slice(&[0x60, 0x01, 0x01, 0x60, 0x01, 0x01]);
                        } else {
                            block_bytes.push(0x01);
                        }
                    }
                    Opcode::JUMPI => {
                        block_bytes.push(0x57);
                        if self.rng.gen_bool(0.4) {
                            block_bytes.extend_from_slice(&[
                                0x5B,
                                0x60,
                                self.rng.gen(),
                                0x50,
                                0x00,
                            ]);
                        }
                    }
                    Opcode::STOP | Opcode::RETURN => {
                        block_bytes.push(if op == Opcode::STOP { 0x00 } else { 0xF3 });
                        if self.rng.gen_bool(0.3) {
                            block_bytes.extend_from_slice(&[
                                0x60,
                                self.rng.gen(),
                                0x50,
                                0x60,
                                self.rng.gen(),
                                0x50,
                            ]);
                        }
                    }
                    Opcode::JUMPDEST => block_bytes.push(0x5B),
                    Opcode::Other(b) => block_bytes.push(b),
                }
            }

            new_bytecode.extend(block_bytes);
        }

        debug!("Chaotic shuffle applied with seed: {}", self.chaotic_seed);
        new_bytecode
    }
}
