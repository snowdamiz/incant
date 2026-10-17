# Portable animated vertex Metal evidence

This data-only packet preserves 91 exact records from the completed `d053542e`
CPU/Metal proof. The gzip archive is about 1.1 MiB. It includes the original
frozen source/tool inventory and scripts as inert UTF-8 data, actual Cargo/test
logs and artifact records, owner/results/terminal absence receipts, raw RGBA,
original PNGs, and root's independent review. No executable/debug artifact is
inside the archive. Four representative original PNGs are also available beside
the archive for inspection.

Run the reviewed verifier with ordinary Python; it only parses/hashes data and
materializes bounded raw pixels/proofs in a temporary directory. It never imports
or executes archived scripts, a binary, or a GPU API:

```
python3 -B verify.py
python3 -B -m unittest -v test_verify
```

When the original copied executable is available, additionally verify its bytes
without running it:

```
python3 -B verify.py --artifact /Users/andreyyurlov/.codex/incant-resume-20261017/animated-vertex-gpu-001-j4_bxkkk/game-tests
```

Portable checks replay recorded pixel/chronology outcomes and the complete
retained command/cleanup record chain. They do not freshly observe process
absence, recover unavailable external tool contents, rerun native code, or prove
authenticity against wholesale fabricated evidence. The original full packet
remains external; its exact source, tools, process absence and binary were
independently checked while present.

The qualified result is fixed-state Metal correctness on Apple M5 Pro: position
replacement matches a fresh changed-asset game at the same clock and changes
visible silhouette; generated tangent handedness replacement with identical
geometry/material/image changes shading and matches its fresh reference. The
same live VM/world/clock continues, fresh motion history is installed, and stale,
late and applicable topology refusals preserve accepted pixels. Source/cache
deletion precedes publication. The prior clip-key test contract also passes.
There is no shipping timing, general appearance, temporal reconstruction,
editor-window workflow or phase-gate claim. Actual editor qualification is a
separate ongoing packet with its own producer and grants.
