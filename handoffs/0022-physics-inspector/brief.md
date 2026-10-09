# Follow-up: shared annotations and one semantic correction

Your d230009 is integrated as 02c7218. Astra has now added the requested units,
mask widget metadata and field order in the Rust registry, regenerated schemas,
and expanded the typed FieldSchema number/array metadata. Runtime/core has also
removed duplicate validation/snapshot work, with unchanged final simulation state.

Correction to the initial brief: **linear and angular damping are rates in 1/s**,
not dimensionless. Rapier applies velocity/(1 + dt*damping). The generated schema
now has accurate descriptions and 1/s units. Fix the fallback profile descriptions
and refresh the fixture to include the current schema annotations, so the next
browser review matches native spacing. Remove redundant hints when sensible.
One-based collision group labels are correct. No native play controls exist yet;
headless CLI motion is the supported runtime evidence, native is Inspector/static
viewport review only.

Also address a correctness point found during review: sectionKeys currently
reorders fields according to profile keys even when the schema supplies an order,
contrary to its documented precedence. Preserve the supplied field order while
applying compatible grouping, including a small behavior test with a reordered
schema. Do not merely change the comment to waive schema precedence.

Run relevant UI checks and review updated body-unit pixels at wide/minimum.
Append result.md, commit Built-by: claude. Native CUA captures will follow after
integration; do not claim native acceptance yet. Do not rebuild unchanged Rust
or regenerate the unchanged motion fixtures. Worktree has the latest integration.

---

# Physics Inspector presentation and real-engine look-dev

Claude Opus 5.5 through ACP owns all visual decisions. Astra has implemented
Rapier physics correctness and the shared schema/command integration. Read
CLAUDE.md and docs/spikes/physics-runtime.md. This is scoped ongoing work, not a
completed phase. Preserve the director's neutral charcoal palette, continuous
panels, titlebar alignment, dedicated left Assets workspace, main Inspector asset
details, and bottom-only Problems/Console/History. The bottom assets layout was
rejected as messy and unprofessional; do not reintroduce it.

## Implement

1. FieldSchema now has a `tagged-union` variant with a string `discriminator` and
   a map of variants. The native bridge decodes only unambiguous required string
   const tags in JSON Schema oneOf; Rust validation remains authoritative. Add
   read-only FieldView support. Select a variant only when the value is a record
   with a recognized tag. Unknown/missing tags must show a clear mismatch, never
   silently select a default. Preserve full nested diagnostic paths, keyboard
   focus and accessible labels. New Collider.shape values are box/half_extents,
   sphere/radius, capsule/half_height/radius. Avoid raw JSON dumps or dense nests.
2. Polish presentation of RigidBody (motion, gravity scale, damping, sleep, CCD),
   Collider (shape, density/friction/restitution, sensor and two 32-bit masks),
   AngularVelocity. These are read-only authored fields. Choose sensible grouping,
   spacing, labels and visual hierarchy. No new authoring controls or mutation
   path. Semantic units: meters, kg/m³ density, rad/s angular velocity; scale and
   coefficients are dimensionless. A mask of 4294967295 means every collision
   group, 0 means none. Do not hide actual values or make up capabilities.
3. Add behavior tests for recognized variants, wrong/missing tags, diagnostics
   and narrow layouts. Run UI tests/build. Review pixels at 1440x900 and 1000x650.
   No new dependencies or remote fonts; JS budget 110 KiB gzip (current102.14).
   Native bridge/contract/schema correctness belongs to Astra: report required
   changes instead of duplicating conversion or validation in the UI.
4. Prepare a small real-engine physics look-dev project if useful for motion
   review, using imported GLB/glTF static geometry and actual colliders. All
   project mutations via incant_cmd/CLI RPC. Physics roots must have unit scale;
   bake visual dimensions into mesh geometry and match Collider dimensions.
   Support is box/sphere/Y-capsule, fixed/dynamic/velocity-kinematic, sensors,
   collision masks and CCD. No character controller, mesh collider, hierarchical
   bodies or rollback claims. Use the actual CLI `play` to capture a short fall
   onto a floor; include an authored camera if needed. Review actual output,
   never substitute fake physics or generated imagery. Source and result may
   live in ignored artifacts. Native verification will follow from Astra.

Paths: editor/ui/src/components/inspector/FieldView.tsx and associated tests/CSS;
editor/bridge/contract.ts and native.ts describe the new read-only union contract;
schemas/{Collider,RigidBody,AngularVelocity}.schema.json are Rust-generated;
crates/incant_physics and incant_script own runtime semantics. Tools:
`npm ci`, `npm run test --workspace editor/ui`, `npm run build --workspace editor/ui`,
`./tools/cargo build -p incant_headless --release --locked`,
`target/release/incant_headless --help`, `node tools/build_script.mjs input.ts output.js`.
Use your worktree's target; never share CARGO_TARGET_DIR.

Native automation/screenshots must use CUA; this ACP session lacks native CUA,
so leave specific requests for Astra. Browser fixtures are permitted for UI
review. Do not read credentials, change accounts, publish or merge. Current
checks: 6 new physics, 2 script and1 command-bus tests pass; workspace Clippy and
295 UI tests/build pass. Full workspace, target execution and native review are
pending. Backend choice is PLAN.md's Rapier fallback, documented in ADR0003.

Return result.md with exact model/transport, changes, tests/pixel evidence,
remaining native requests and a scoped verdict. Commit with Built-by: claude.
