# Shipping player probe library and executable audits

Checkpoint17's macOS `checks` job completed the standalone compiler build, then `native-game.py` rejected its evidence because the library and executable share `incant_game_player` as a crate name. The unchanged original loop reproduces the exact failure against the retained hosted `build-2.stderr.log`.

The native game and native window probes now independently require one library and one executable with exact selected target/features. Both must use the shipping optimization, codegen, panic, debug and split-debug settings; the final executable must use fat LTO. Source compiler, test-harness and host-native CPU flags remain refused. Cargo's executable selection also explicitly requires the binary kind. No count threshold was relaxed and no runtime code changed.

The audit of other shipping helpers found that desktop export and the fresh-session ledger already distinguish kinds; mobile builds explicitly request and verify a static library; the script player is binary-only; the static Wasm player verifies its two units and unique binary. Package graph and build-script symbol-manifest counts describe package/build identities, not library/executable units. Exact audited file hashes and search results are retained.

Six regression controls and the corrected actual hosted-log audit pass. Full tools pass 355 Python tests with four existing Windows-only skips and 22 Node tests; conventions and diff checks pass. The [machine-readable packet](evidence/shipping-player-probe-kinds.json) retains the original failure, successful compile records, red/green replay and all check outputs. Independent final source/evidence review accepted the exact four inputs and all retained records; the required fast check remains pending.

Checkpoint17 remains unpromoted with 20 required successes and three failures. The Linux/Windows quota assertion has a separate main188 test-only repair. The next frozen release checkpoint must execute every one of the existing 23 requirements successfully; re-auditing the old compile log does not claim a new shipping run.
