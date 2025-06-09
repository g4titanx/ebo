/// module for parsing and analyzing evm bytecode in the ebo obfuscator.
/// provides functionality to split bytecode into basic blocks and compute control flow graph (cfg)
/// complexity, supporting obfuscation techniques and reverse engineering resistance tests.
use std::collections::HashSet;

/// represents an evm opcode, used to categorize instructions during bytecode parsing.
/// variants cover key control-flow and arithmetic opcodes relevant to obfuscation, with a fallback
/// for unrecognized instructions.
#[derive(Debug, PartialEq, Clone)]
/// draws on research from eveilm (page 47) and bosc (table i) for cfg complexity metrics.
pub enum Opcode {
    /// addition operation (0x01), targeted for substitution in obfuscation (eveilm, page 59).
    ADD,
    /// conditional jump (0x57), used in false branch obfuscation (bosc, section 2.2).
    JUMPI,
    /// jump destination (0x5b), inserted in false branches (bosc, section 2.2).
    JUMPDEST,
    /// stop execution (0x00), marks unreachable code regions for flower instructions (bosc, section 2.4).
    STOP,
    /// return from execution (0xf3), marks unreachable code regions (bosc, section 2.4).
    RETURN,
    /// unrecognized or other opcode, stored as its byte value.
    Other(u8),
}

/// represents a basic block of evm bytecode, a sequence of opcodes executed sequentially.
/// used to isolate code segments for chaotic shuffle and other obfuscation techniques (bian, section iii.b).
#[derive(Debug, Default)]
pub struct BasicBlock {
    /// sequence of opcodes within the block.
    pub opcodes: Vec<Opcode>,
}

/// parses evm bytecode into a vector of basic blocks.
/// splits bytecode at control-flow opcodes (jumpi, jumpdest, stop, return) to create independent
/// segments for obfuscation, ensuring safe manipulation of non-control instructions (bian, section iii.b).
///
/// # arguments
/// * `bytecode` - slice of raw evm bytecode bytes.
///
/// # returns
/// vector of `BasicBlock` instances, each containing a sequence of opcodes.
///
/// # example
/// ```
/// let bytecode = vec![0x60, 0x01, 0x01, 0x57, 0x00]; // PUSH1 1, ADD, JUMPI, STOP
/// let blocks = parse_bytecode(&bytecode);
/// assert_eq!(blocks.len(), 2); // Two blocks: [PUSH1, ADD, JUMPI], [STOP]
/// ```
pub fn parse_bytecode(bytecode: &[u8]) -> Vec<BasicBlock> {
    let mut blocks = Vec::new();
    let mut current_block = BasicBlock {
        opcodes: Vec::new(),
    };
    let mut i = 0;

    while i < bytecode.len() {
        let op = match bytecode[i] {
            0x01 => Opcode::ADD,
            0x57 => Opcode::JUMPI,
            0x5B => Opcode::JUMPDEST,
            0x00 => Opcode::STOP,
            0xF3 => Opcode::RETURN,
            b => Opcode::Other(b),
        };

        current_block.opcodes.push(op.clone());

        // after a control-flow opcode (JUMPI, JUMPDEST, STOP, or RETURN) is encountered, the current
        // BasicBlock (stored in current_block) needs to be moved into the blocks vector, and a new empty
        // BasicBlock needs to be prepared for the next segment
        if matches!(
            op,
            Opcode::JUMPI | Opcode::STOP | Opcode::RETURN | Opcode::JUMPDEST
        ) {
            blocks.push(std::mem::take(&mut current_block)); // to avoid unnecessary cloning and reallocations

            // since loop will keep appending new opcodes to `current_block.opcodes` for the next segment, we
            // need to ensure `current_block` is properly initialized for the next iteration, else we might
            // end up with unexpected behavior (e.g., reusing a partially filled or uninitialized state), hence
            // why we have another assignment below.
            current_block = BasicBlock::default();
        }

        i += 1;
    }

    if !current_block.opcodes.is_empty() {
        blocks.push(current_block);
    }

    blocks
}

/// computes a simple control flow graph (cfg) complexity metric for a set of basic blocks.
/// measures the number of blocks containing a jumpi (0x57) instruction, serving as a proxy for
/// reverse engineering difficulty (eveilm, page 47; bosc, table i).
///
/// # arguments
/// * `blocks` - slice of `BasicBlock` instances parsed from bytecode.
///
/// # returns
/// number of blocks containing at least one jumpi instruction, indicating cfg branching complexity.
///
/// # example
/// ```
/// let bytecode = vec![0x01, 0x57, 0x00]; // ADD, JUMPI, STOP
/// let blocks = parse_bytecode(&bytecode);
/// let complexity = compute_cfg_complexity(&blocks);
/// assert_eq!(complexity, 1); // One block with JUMPI
/// ```
#[allow(dead_code)]
pub fn compute_cfg_complexity(blocks: &[BasicBlock]) -> usize {
    blocks
        .iter()
        .filter(|b| b.opcodes.iter().any(|op| matches!(op, Opcode::JUMPI)))
        .count()
}

/// counts the number of unique opcodes in a bytecode slice.
/// used as a readability metric to assess obfuscation’s impact on reverse engineering difficulty,
/// where more unique opcodes indicate increased complexity (eveilm, page 59).
///
/// # arguments
/// * `bytecode` - slice of raw evm bytecode bytes.
///
/// # returns
/// number of unique opcode bytes in the bytecode.
///
/// # example
/// ```
/// let bytecode = vec![0x60, 0x01, 0x01, 0x57]; // PUSH1, ADD, ADD, JUMPI
/// let unique_count = count_unique_opcodes(&bytecode);
/// assert_eq!(unique_count, 3); // PUSH1, ADD, JUMPI
/// ```
#[allow(unused)]
pub fn count_unique_opcodes(bytecode: &[u8]) -> usize {
    let mut unique = HashSet::new();
    for &b in bytecode {
        unique.insert(b);
    }
    unique.len()
}

/// computes a simplified halstead’s effort proxy for bytecode analysis complexity.
/// estimates the effort required to understand bytecode by combining unique opcodes (operators)
/// and total instructions (operands), inspired by eveilm’s halstead metrics (page 59). higher effort
/// indicates greater difficulty for reverse engineering.
///
/// # arguments
/// * `bytecode` - slice of raw evm bytecode bytes.
///
/// # returns
/// floating-point value representing the estimated analysis effort (n1 * n2 * log2(n2)).
///
/// # example
/// ```
/// let bytecode = vec![0x60, 0x01, 0x01]; // PUSH1, ADD, ADD
/// let effort = halstead_effort_proxy(&bytecode);
/// assert!(effort > 0.0); // Effort scales with opcode count and variety
/// ```
#[allow(unused)]
pub fn halstead_effort_proxy(bytecode: &[u8]) -> f64 {
    let n1 = count_unique_opcodes(bytecode) as f64; // Unique operators
    let n2 = bytecode.len() as f64; // Total operands
    
    n1 * n2 * n2.log2() // Simplified effort
}
