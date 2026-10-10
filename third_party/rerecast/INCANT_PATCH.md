# Incant patch to rerecast 0.4.0

Original: crates.io `rerecast = 0.4.0`, archive SHA-256
`d91bc1e22832298b01bca704eab9d6fe1d1c8c9cf6a1a44554e678b807884360`.
The packaged `.cargo_vcs_info.json` identifies upstream commit
`5ac1afadd64ae100052fe5b779a2b6d43f1aff82`, path `crates/rerecast`.
MIT license selected; `LICENSE-MIT` is copied from Incant's retained upstream
v0.4.0 license. The registry's `.cargo-ok` cache marker is omitted.

Changes by Incant on 2026-10-10:

- `src/rasterize.rs`: clear both output counts when `divide_poly` receives zero
  input vertices. The old early return retained counts from an earlier row;
  later clipping could rasterize solid spans outside the triangle's footprint.
  Real capsule facets reproduced a missing 0.6 by 0.6 m walkable floor patch.
  `crates/incant_nav/tests/rasterization.rs` guards the public rasterizer behavior.
- `src/detail_mesh.rs`: remove the deprecated `core::f32` module import so `MAX`
  resolves to the primitive associated constant on the current Rust toolchain.

No other library algorithm or manifest changes are made. Incant's workspace
excludes this dependency from its member list and selects this exact source
through `[patch.crates-io]`. Keep this patch until an audited upstream release
contains the correction; do not edit Cargo's shared registry cache.
