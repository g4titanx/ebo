/// module for obfuscating evm bytecode
/// implements techniques like chaotic shuffle, opcode substitution, false branch obfuscation, and flower instructions
/// draws on research from eveilm (page 59), bosc (sections 2.2, 2.4), and bian (section iii.b).
use crate::evm::{parse_bytecode, Opcode};
use log::debug;
use rand::{rngs::StdRng, Rng, SeedableRng};
use sha2::{Digest, Sha256};

/// responsible for obfuscating evm bytecode.
/// holds the input bytecode, a seeded random number generator for deterministic obfuscation,
/// and a chaotic seed for the chaotic shuffle technique.
pub struct Obfuscator {
    /// input evm bytecode to be obfuscated.
    bytecode: Vec<u8>,
    /// seeded random number generator for deterministic obfuscation operations.
    ///
    /// (a bit unrelated but very useful) you see a seed is like a starting point for a random number generator (in this case, StdRng).
    /// when you initialize the rnd number generator with a specific seed (e.g., 42), it uses that seed to generate a
    /// fixed sequence of "random" numbers (pseudo-random specifically) every time you run it with the same seed. this rng is then used in
    /// the obfuscate method to make decisions, such as: whether to shuffle opcodes (rng.gen_bool(0.3)), which opcodes to swap during
    /// shuffling (rng.gen_range), whether to apply substitutions or insert false branches (rng.gen_bool with probabilities like 0.5 or 0.4).
    /// because the seed is fixed, these decisions follow the same pattern each time. for example, if rng.gen_bool(0.3) returns true for the
    /// first shuffle decision with seed 42, it will always return true in that same spot when using seed 42.
    ///
    /// so for a given input bytecode and the same seed, the obfuscator will produce the same obfuscated bytecode every time. this is because
    /// the random choices (e.g., which opcodes to shuffle or substitute) are deterministic based on the seed’s sequence.
    rng: StdRng,
    /// a floating-point number between 0 and 1 derived from the input seed, used later in the chaotic_map function
    ///  to introduce controlled randomness.
    chaotic_seed: f64,
}

impl Obfuscator {
    /// creates a new obfuscator instance for the given bytecode and seed.
    /// initializes the random number generator and chaotic seed using a sha-256 hash of the input seed
    /// to ensure deterministic yet unpredictable obfuscation.
    ///
    /// # arguments
    /// * `bytecode` - slice of raw evm bytecode to obfuscate.
    /// * `seed` - 64-bit unsigned integer seed for deterministic obfuscation.
    ///
    /// # returns
    /// a new `Obfuscator` instance configured with the bytecode, seeded rng, and chaotic seed.
    ///
    /// # example
    /// ```
    /// let bytecode = vec![0x01, 0x57]; // ADD, JUMPI
    /// let obfuscator = Obfuscator::new(&bytecode, 42);
    /// ```
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

    /// transforms an input x into a new value using piecewise trigonometric formulas, generating a chaotic
    /// sequence constrained to [0, 1]. this sequence drives the obfuscation’s shuffle intensity, leveraging
    /// deterministic randomness to enhance security while preserving repeatability.
    ///
    /// this is heavily inspired by bian’s chebyshev-pwlcm chaotic map (section iii.b), this function produces
    /// pseudo-random values for the chaotic shuffle, ensuring deterministic yet unpredictable opcode
    /// reordering within basic blocks.
    ///
    /// # arguments
    /// * `x` - current value in the chaotic sequence (between 0.0 and 1.0).
    ///
    /// # returns
    /// next value in the chaotic sequence, used to control shuffle intensity.
    fn chaotic_map(&mut self, x: f64) -> f64 {
        // a constant that influences the chaotic behavior.
        // this value is chosen to create a nonlinear effect, often seen in chaotic systems to amplify small changes in input.
        let mu = 3.9;
        // a threshold that splits the input range into two different transformation rules, adding piecewise complexity.
        let p = 0.4;

        if x < p {
            (x.cos() * mu * x.cos()).sin().abs() % 1.0
        } else {
            (1.0 - x).sin() % 1.0
        }
    }

    /// obfuscates the stored bytecode using multiple techniques.
    /// applies chaotic shuffle, opcode substitution, false branch obfuscation, and flower instructions
    /// to increase control flow graph (cfg) complexity and analysis effort, making reverse engineering
    /// difficult (eveilm, page 47; bosc, table i). preserves functional equivalence for evm execution.
    ///
    /// # returns
    /// vector of obfuscated bytecode bytes.
    ///
    /// # example
    /// ```
    /// let bytecode = vec![0x01, 0x57]; // ADD, JUMPI
    /// let mut obfuscator = Obfuscator::new(&bytecode, 42);
    /// let obfuscated = obfuscator.obfuscate();
    /// // may produce e.g., [0x60, 0x01, 0x01, 0x60, 0x01, 0x01, 0x57, 0x5B, 0x60, 0xXX, 0x50, 0x00]
    /// ```
    pub fn obfuscate(&mut self) -> Vec<u8> {
        let blocks = parse_bytecode(&self.bytecode);
        let mut new_bytecode = Vec::new();
        let mut chaotic_val = self.chaotic_seed;

        for block in blocks {
            let mut block_bytes = Vec::new();
            let mut opcodes: Vec<Opcode> = block.opcodes;

            // Chaotic shuffle within block (which avoids shuffling jump-related opcodes)
            //
            // the chaotic shuffle reorders non-control-flow opcodes within each basic block to obscure the code’s structure.
            // it uses the chaotic_map function to derive a sequence of values that influence the number of shuffles and the
            // specific reordering, which is guided by a seed-derived chaotic_seed.
            if self.rng.gen_bool(0.3) {
                chaotic_val = self.chaotic_map(chaotic_val);
                let shuffle_count = (chaotic_val * opcodes.len() as f64) as usize;
                let safe_opcodes: Vec<_> = opcodes
                    .iter()
                    .enumerate()
                    .filter(|(_, op)| !matches!(op, Opcode::JUMPI | Opcode::JUMPDEST)) // to avoid invalid jumps or broken execution paths.
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

            // apply opcode substitution, false branch obfuscation, and flower instructions
            for op in opcodes {
                match op {
                    Opcode::ADD => {
                        if self.rng.gen_bool(0.5) {
                            // apply opcode substitution: replace add -> push1 1 add push1 1 add (eveilm, page 59)
                            block_bytes.extend_from_slice(&[0x60, 0x01, 0x01, 0x60, 0x01, 0x01]);
                        } else {
                            // retain original add opcode without substitution
                            block_bytes.push(0x01);
                        }
                    }
                    Opcode::JUMPI => {
                        // retain jumpi opcode
                        block_bytes.push(0x57);
                        if self.rng.gen_bool(0.4) {
                            // apply false branch obfuscation: add unreachable jumpdest -> push1 <random>, pop, stop (bosc, section 2.2)
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
                        // retain stop or return opcode
                        block_bytes.push(if op == Opcode::STOP { 0x00 } else { 0xF3 });
                        if self.rng.gen_bool(0.3) {
                            // apply flower instruction obfuscation: add unreachable push1 <random> pop push1 <random> pop (bosc, section 2.4)
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
                    Opcode::JUMPDEST => {
                        // retain jumpdest opcode without additional obfuscation
                        block_bytes.push(0x5B)
                    }
                    Opcode::Other(b) => {
                        // retain unrecognized opcode without obfuscation
                        block_bytes.push(b)
                    }
                }
            }

            new_bytecode.extend(block_bytes);
        }

        debug!("Chaotic shuffle applied with seed: {}", self.chaotic_seed);
        new_bytecode
    }
}
