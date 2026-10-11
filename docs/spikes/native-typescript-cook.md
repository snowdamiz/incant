# Bounded native TypeScript cooking

The developer native-play coordinator now accepts a TypeScript entry with local
module imports. It strictly typechecks the captured text, emits CommonJS and a
flat version-3 source map, then uses the existing trusted QuickJS bytecode build.
The copied native process runs after the temporary source/map inputs are removed.
This is a locally verified authoring prerequisite; editor Play and project-attached
`Script` components still require migration.

```sh
python3 tools/native_play.py /path/to/game.incant.json \
  --typescript scripts/game.ts --ticks 120

# Or emit reviewable cook inputs into a new directory without building a player:
node tools/build_native_script.mjs /path/to/project scripts/game.ts /path/to/new-cook
```

`--script-root` overrides the default project-directory root. The entry is relative
to that root (an absolute entry must also be inside it). The existing `--script`
CommonJS path remains available; it is mutually exclusive with `--typescript`.
`--source-map` applies only to explicit CommonJS input.

The entry exports one `NativeBehavior`, normally using
`import type { NativeBehavior } from '@incant/runtime/native'` and
`export default { update(context) { /* bulk work */ return undefined; } } satisfies NativeBehavior`.
That SDK name is a cook-time type alias, not a runtime package loader. No JavaScript
source, TypeScript compiler, Node process or engine-agent shell capability is added
to the native gameplay tick.

## Input and execution boundary

- At most 64 local `.ts` modules, 256 KiB per file and 1 MiB of total source;
  the final CommonJS bundle retains the existing 1,000,000-byte native limit and
  the source map the existing 4 MiB limit.
- Static relative imports/reexports support explicit `.ts`, extensionless files
  and `index.ts`. Ambiguous file/directory resolutions fail. Local cycles share
  one cached CommonJS factory result. Type-only imports do not run the imported
  factory; pure type-only source maps may have no generated segments.
- External package/Node/network imports, dynamic imports, import attributes,
  `import.meta`, CommonJS declarations, project declaration files, decorators and
  reference directives fail explicitly. The identifiers `require`, `exports` and
  `module` are reserved by this initial module cook.
- The graph rejects path escapes and symbolic-link components, checks regular
  files and opened-file identities, bounds reads, requires UTF-8 and checks size,
  timestamp and canonical path after reading. Compilation uses captured text.
  These checks do not claim an OS sandbox against a concurrently hostile local
  process replacing directories between filesystem operations.
- Strict TypeScript uses only captured source, the repository native/generated
  declarations and pinned standard libraries. Project `tsconfig`, package scripts,
  plugins and SWC configuration are not loaded. SWC transforms text without
  executing it. Runtime factories execute later inside the hardened native VM.
- The coordinator clears ambient Node injection options, caps its compiler heap
  at 512 MiB and its process at 60 seconds. The standalone compiler CLI is a trusted
  developer tool; its caller owns process resource limits.
- Output is create-only, with private directory/file modes and the manifest
  written last. Consumers use it only after successful completion. Write failures
  remove only matching objects created by that invocation; unrelated/replaced
  objects survive, and incomplete cleanup is reported. This is not an atomic
  directory snapshot for arbitrary concurrent readers.

The manifest hashes every original module, bundle and map and records the pinned
TypeScript 5.9.3/SWC 1.16.13 versions. Module labels in maps are relative, and
flat mappings retain source/name indices and generated wrapper offsets. These
hashes support identity checks, not authentication of externally supplied bytecode.
Source maps remain build symbols; automatic runtime stack remapping is still open.

## Local verification

Source `519c975` passes 12 Node tests and all 90 Python tool tests. Coverage includes
strict errors and missing entry contracts; shared imports, cycles, reexports and
type-only modules; source-map locations and byte repeatability; ignored project
configuration; containment, links, UTF-8 and size/count limits; existing-output
refusal; injected second/third partial-write failures; successful retry and
preservation of unrelated files. Node fixture execution tests bundle semantics,
not the native sandbox.

The real `native-project-play.py --typescript` probe cooked four modules, including
a type-only dependency, deleted all four source files and temporary compiler inputs,
and ran six fresh copied-native processes: three identical successful input/motion/
Unicode-log runs, one completed false assertion, one terminal script failure without
published output, and one wrong-script-hash refusal. Project/journal hashes remained
unchanged. The actual VM also refused dynamic source compilation routes. The bundle
SHA256 is `88763acdfbdcd42842f65c1f81e7e67cffd4b52243b6a14da5cf974ae3771b1b`;
the developer executable SHA256 is
`fd804e3a349c0ab30afc5f172a515c5091bf7f43a80ec7dda14a3e94ea645a26`.

Independent Astra source review found and verified the correction to partial-output
cleanup. Its reviewed file hashes and the complete process result are retained in
[local evidence](evidence/native-typescript-cook-local.json). The probe uses a release
developer executable that still includes authoring tools; it is not a fresh shipping
package, mobile export, performance measurement, complete SDK or phase-gate result.

New work stays local until the coordinated runtime queue is integrated. Hosted
checks, full project-script routing, editor Play, full SDK, cross-target export and
the plan's shipping/reference-phone gates remain open.
