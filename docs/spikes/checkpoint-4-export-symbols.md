# Checkpoint 4: authored exporter MSVC symbol audit

The frozen main94 Windows desktop job [114544649458](https://github.com/snowdamiz/incant/actions/runs/38164674056/job/114544649458) passed the formerly failing authored TypeScript native-agent test, full ordinary workspace tests and source-transaction probe. It then built the fresh authored shipping player, but the export audit failed with `Missing bytecode runtime`: it used `llvm-nm` on the linked PE image, whose bytecode symbols are not present in a COFF symbol table. The copied shipping player never launched. The 36m38s failed job is retained evidence, not release qualification.

The correction follows the existing Windows wasm2c proof: invoke `cargo rustc` for the explicit final `incant_game_player` binary with a new private `/MAP` output. Audit the fresh linker map for both bytecode functions and every existing forbidden source-parser symbol; also recognize the Rust compiler module's mangled form. The map's exact filename/source and SHA-256 enter the export receipt, and its file stays beside developer evidence outside the player directory. Required shipping flags, compiler-disabled target audit, normal dependency graph, matched PE/PDB identity, private build deletion and subsequent copied-player checks remain unchanged. Mach-O/ELF still use defined, demangled `llvm-nm` output.

Microsoft documents the map's final-link module, addresses and public symbol definitions in its [/MAP reference](https://learn.microsoft.com/en-us/cpp/build/reference/map-generate-mapfile?view=msvc-170). No global Rust flags, dependency caches, workflow command coverage or release contexts change.

## Verification

- An actual local cross-linked PE/PDB fixture has empty `llvm-nm` output (the old check fails), a map containing the required bytecode functions, and matching PE/PDB GUID and age. The new audit passes; a parser symbol is rejected. This uses Apple Clang 21 and the pinned Rust toolchain's LLD 23; it is not actual Windows execution.
- Eight package tests pass, including missing maps/functions, every forbidden parser symbol, both Rust manglings, and exact map identity. The full Python suite runs 210 tests: 206 pass and four platform-specific cases skip. Conventions and diff checks pass.
- A fresh actual macOS authored export passes shipping/codegen/symbol/normal-graph audits and matched dSYM packaging. After build and authoring deletion, three copied-player processes each run 20 ticks and reproduce X=2.0 within 1e-9. Player SHA-256: `65dac69da75aaa11a8e23242d1cd1af1cf639cb1f9dcf7abb94635572f480239`.
- Independent source/fixture review accepted the bounded fix with no finding. The runtime and profile are unchanged; full Windows exporter execution remains required in the next frozen checkpoint.

[Exact sources, review, failure and proof receipts](evidence/checkpoint-4-export-symbols.json). No device, performance, GPU, signing, complete export or phase gate is approved.
