#[derive(Debug, PartialEq, Clone)]
pub enum Opcode {
    ADD,
    JUMPI,
    JUMPDEST,
    STOP,
    RETURN,
    Other(u8),
}

#[derive(Debug, Default)]
pub struct BasicBlock {
    pub opcodes: Vec<Opcode>,
}

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

        if matches!(
            op,
            Opcode::JUMPI | Opcode::STOP | Opcode::RETURN | Opcode::JUMPDEST
        ) {
            blocks.push(std::mem::take(&mut current_block));
            current_block = BasicBlock {
                opcodes: Vec::new(),
            };
        }

        i += 1;
    }

    if !current_block.opcodes.is_empty() {
        blocks.push(current_block);
    }

    blocks
}

// Simple CFG complexity metric (number of branches)
#[allow(dead_code)]
pub fn compute_cfg_complexity(blocks: &[BasicBlock]) -> usize {
    blocks
        .iter()
        .filter(|b| b.opcodes.iter().any(|op| matches!(op, Opcode::JUMPI)))
        .count()
}
