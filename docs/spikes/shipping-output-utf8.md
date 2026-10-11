# Shipping Cargo output encoding

PR 57 head `2512ff0ee902c33779114c8f847379bca09eebc1` failed in Windows
job 114332881615 (run 38092910783) after its Rust tests and editor/tool build.
`tools/shipping.py` captured Cargo JSON with locale-default text decoding.
Python 3.12 selected cp1252, rejected byte 0x81 at offset 1,731,915, and its
Windows reader thread left stdout unset; artifact parsing then raised an
AttributeError. The shipping executable and symbol checks did not run. This
log does not establish a runtime failure or a successful shipping build.

The shared `cargo_output` helper decodes both captured Cargo streams explicitly
as UTF-8, with strict error handling and unchanged exit status. Shipping runtime,
script and audio builds all use it; script/audio build logs are written as UTF-8.
No build flags, cache keys, execution assertions or retries change.

A real child-process regression emits a JSON artifact path containing `ā` (whose
UTF-8 includes cp1252-invalid byte 0x81) and Unicode stderr. It controls only the
parent subprocess locale lookup. The regression failed with UnicodeDecodeError
before the explicit encoding change and passes afterward; artifact path and
stderr remain exact. A second process exits 17 and retains that failure status.
`python3 -m unittest discover -s tools/tests` passes all 40 tests; conventions
and diff checks pass. These local subprocess tests reproduce the decoding bug,
not the complete hosted Windows packaging run; corrected Windows CI remains
required. Runtime sources and dependencies are unchanged, so local Rust rebuilds
would not exercise this correction.
