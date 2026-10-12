# Real C and Swift native runtime callers

The frozen macOS `mobile-ffi` static archive now runs the same typed cooked game
through actual C and Swift callers importing `incant_game.h`. Independent source
and execution review accepted this scope. The ordinary native input path moves
entity 1 to x=0.1 after three held-input ticks, and x=0.05 when released at tick2;
all 32 controls across both callers pass. PCM and repeated complete reports agree.

The 39 recorded commands include real linking and processes, exact guest and
input-I/O failures, completed failed observations, package checksum refusal,
malformed/null/oversized requests, and create-only output preservation. A nonzero
exit alone cannot satisfy a negative control. The first run failed its helper's
exact diagnostic spelling; correction to `script tick failed` left the engine and
archive unchanged. That run remains in the evidence history.

The fixture producer uses `CookedScene` APIs, not the historical document-oriented
platform smoke. Its isolated Cargo lock retains the engine's dependency versions,
checksums and checked navigation patch; the normal runtime graph stays authoring-
free. Fixture build, all-target Clippy, formatting and four harness rejection
unit tests pass. Only documentation and an additional unit test changed after the
final actual run; the manifest identifies both explicitly.

[The evidence manifest](evidence/mobile-runtime-callers.json) binds the archive,
header, fixture generator, linked executables, source inputs and all raw/retained
command data. The immutable archive is 86,391,472 bytes under the `ci` profile;
that number is an artifact identity, not a shipping-size gate. The accepted
[independent review](evidence/mobile-runtime-callers/review.json) remains scoped
to macOS CPU execution of the default trusted build-linked fixture.

The same caller tool has an explicit `linked_guest` contract for the separately
reviewed numerical compatibility fixture: source/expectations hashes, complete
15-message held/released log vectors with ticks/levels, and actual native motion.
That path has not yet executed through these callers. iOS archive production hit
the existing cook target guard; the compatibility owner is resolving it separately.
The headless disposable-simulator route and iOS device link route are implemented,
but neither is execution evidence yet. The Android NDK archive/C-link helper also
preserves that guard and has not run; no known-failing job is presented as a
successful Android portability gate.

Next work is the accepted numerical target archive followed by actual simulator
calls, then the separately admitted Android producer and candidate-only CI step.
No native UI, display, physical phone, hardware audio, authored mobile export,
frame-time or complete game-release claim follows from this checkpoint.
