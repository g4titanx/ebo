# ebo: evm bytecode obfuscation

`ebo` is a cli tool designed to obfuscate EVM bytecode, enhancing smart contract security by complicating reverse engineering efforts while preserving the original functionality. the tool employs a suite of obfuscation techniques, at the moment, it contains a chaotic shuffle inspired by the Chebyshev-PWLCM chaotic map from [BiAn](https://yanxiao6.github.io/papers/BiAn.pdf), which deterministically reorders non-control-flow opcodes within basic blocks using a user-specified seed (default: 42). additionally, `ebo` implements opcode substitution, replacing simple instructions such as `ADD (0x01)` with equivalent sequences (e.g., `PUSH1 1 ADD PUSH1 1 ADD`), introduces false conditional branches via `JUMPI (0x57)` and `JUMPDEST (0x5B)` to disrupt control flow analysis, and inserts flower instructions (e.g., PUSH1 <random> POP, 60xx50) in unreachable code regions to increase complexity. the command-line interface, structured as `ebo obfuscate --file <path> --seed <seed> --verbosity <level>` (for testing `RUST_LOG=debug ./target/release/ebo obfuscate --file examples/incrementer.bin --seed 42 --verbosity verbose`), accepts a bytecode file input (e.g., incrementer.bin), generates an obfuscated output in obfuscated.bin, and provides verbose logging of the original and obfuscated bytecode alongside metrics like length increase (approximately 32% for the Incrementer contract, from 328 to 435 bytes).

this is an active experimental workspace, so i'd regularly make updates about what i learn here

## todos

- [ ] **Expand to Source Code Obfuscation**
  - Integrate a Solidity parser (e.g., via `solc` JSON AST) to process `.sol` files, enabling source-level transformations.
- [ ] **Implement Control Flow Obfuscation**
  - Develop opaque predicate insertion (e.g., using CPM or a simplified chaotic map) and implement control flow flattening by restructuring Solidity functions into sequential blocks with dispatch tables.
- [ ] **Implement Data Flow Obfuscation**
  - Add AST-based transformations: convert local to global variables (with checks for `pure`/`view` functions), replace static data with dynamic arrays, convert integers to expressions, split Booleans, and transform scalars to vectors.
- [ ] **Implement Layout Obfuscation**
  - Randomize variable/function names, remove comments, and disrupt formatting in Solidity source code.
- [ ] **Enhance Chaotic Mapping**
  - Implement the full CPM chaotic map (as per BiAn, Page 7) with parameters `μ`, `p`, and `M`, and apply it to opaque predicates and shuffle counts.
- [ ] **Optimize Gas Consumption**
  - Analyze gas usage with `solc --estimate-gas`, implement BiAn’s optimizations (local variables, `view` functions, minimizing loops/strings), and add a config file to balance obfuscation intensity.
- [ ] **Extensive Testing and Validation**
  - Create a diverse Solidity dataset, test against Vandal, Gigahorse, Mythril, Slither, and EVeilM’s tools, and measure complexity (e.g., cyclomatic complexity, Halstead’s Effort) and gas costs.
- [ ] **Address Limitations and Scalability**
  - Support multi-contract files, handle `solc` warnings, and adjust for `CREATE2` opcodes in parent contracts (e.g., UniswapFactory).
- [ ] **Support Vyper and Other EVM-Compatible Languages**
  - Adapt obfuscation logic to parse and obfuscate Vyper or Rust (via WASM) contracts, testing with examples like Listings 2.2 and 2.3.
- [ ] **Formal Verification**
  - Integrate a formal verification tool (e.g., CertiK, Mythril’s formal mode) to validate obfuscated contracts’ behavior.
- [ ] **Inter-Contract Obfuscation**
  - Develop logic to obfuscate data flows between contracts, adjusting for cross-call dependencies.
- [ ] **Hybrid Source/Bytecode Obfuscation**
  - Design a pipeline that applies source-level obfuscation (BiAn-style) followed by bytecode-level tweaks (EVeilM-style), ensuring consistency.
- [ ] **Decompiler-Specific Obfuscation**
  - Analyze decompiler behavior (e.g., Dedaub, Heimdall) and develop targeted obfuscation patterns (e.g., Function Signature Transformer).
- [ ] **User Documentation and Configurability**
  - Create a `README.md` with installation instructions, a `Configuration.json` for toggling features (e.g., obfuscation probability), and examples.