# Native export command lifetime

The public native exporter now runs crash-symbol tools through the existing
`native_process.run` owner. The symbol helper resolves that function through its
module, so a trusted export supervisor's remaining deadline and cancellation
checks also cover symbol packaging. Its default command budget is 120 seconds;
trusted callers can supply a different finite positive timeout or cancellation
callback. Successful and failed tools both finish owned-tree cleanup before the
caller can remove private inputs. Merged UTF-8 output and the nonzero-exit
`ValueError` diagnostic are preserved.

The public exporter also opts into the existing managed translation executor for
authored Wasm. WABT version checking and translation receive the same sanitized
build environment and supervision as capture. The pinned translator flags,
captured-byte identities, cooked data, runtime features and publication behavior
are unchanged. TypeScript projects still skip this translation route.

The change is deliberately narrower than a complete exporter wall-clock bound.
The historical Core window export at `ceda3a94` used synchronous symbol tools,
and its external supervisor's pre/post Git checks and file work remain outside
the managed wait. That run and supervisor are preserved. Standalone `wasm2c.py`
still has its separate default executor; only the public exporter's existing
injected route changes here. Filesystem copying, hashing, kernel termination and
reaping are not promised a hard wall-clock deadline. Captured command output
retains its existing unbounded storage behavior.

Focused Python controls use real harmless Python child/descendant trees for
success, timeout, cancellation, nonzero exit and invalid UTF-8. They require an
actually running sentinel writer, then verify it stopped before returning to the
temporary-directory owner. A separate public-build routing control runs the real
selection/capture/preparation code with simulated trusted tool outputs, forbids
the unmanaged translator helper, checks exact input/prepared hashes and sanitized
environment, and refuses further work after either WABT failure. These are
lifecycle and routing controls, not a new native build or shipped-Wasm execution.

Local qualification passed 11 new controls and 80 existing Python tests, with
four Windows-only lifecycle checks skipped on this Mac. The existing group covers
`test_native_process`, `test_shipping`, `test_wasm2c_prepare`,
`test_native_ui_export`, `test_native_profiling`, `test_ios_wasm` and
`test_android_wasm`. Both bounded test children exited zero; no native compiler,
player, renderer or GPU was executed for this repair.
