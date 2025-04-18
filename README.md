# ebo: evm bytecode obfuscation

`ebo` is a cli tool designed to obfuscate EVM bytecode, enhancing smart contract security by complicating reverse engineering efforts while preserving the original functionality. the tool employs a suite of obfuscation techniques, at the moment, it contains a chaotic shuffle inspired by the Chebyshev-PWLCM chaotic map from [BiAn](https://yanxiao6.github.io/papers/BiAn.pdf), which deterministically reorders non-control-flow opcodes within basic blocks using a user-specified seed (default: 42). additionally, `ebo` implements opcode substitution, replacing simple instructions such as `ADD (0x01)` with equivalent sequences (e.g., `PUSH1 1 ADD PUSH1 1 ADD`), introduces false conditional branches via `JUMPI (0x57)` and `JUMPDEST (0x5B)` to disrupt control flow analysis, and inserts flower instructions (e.g., PUSH1 <random> POP, 60xx50) in unreachable code regions to increase complexity. the command-line interface, structured as `ebo obfuscate --file <path> --seed <seed> --verbosity <level>` (for testing `RUST_LOG=debug ./target/release/ebo obfuscate --file examples/incrementer.bin --seed 42 --verbosity verbose`), accepts a bytecode file input (e.g., incrementer.bin), generates an obfuscated output in obfuscated.bin, and provides verbose logging of the original and obfuscated bytecode alongside metrics like length increase (approximately 32% for the Incrementer contract, from 328 to 435 bytes).

this is an active experimental workspace, so i'd regularly make updates about what i learn here
