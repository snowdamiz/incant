# Preserved Claude0041 reference

`0041-fixtures.py` is copied byte-for-byte from the accepted Claude0041 evidence
commit23fe9cae78de54ad47d50a89b3b8ce9cc7e1a118. SHA256 `46d9237deb0b427aec2d74f57935962bec175e546e3933bfebaba2a318ca9a7f`.
It contains the original fixture artwork/source generator and no engine calls.
Do not edit this reference. Select or adapt only the minimal controls needed for
0043 in a new helper, with explicit attribution. All engine calls must use the
0043 per-subprocess guard; the old0041 runner is deliberately not copied.

Claude0041's shared defect was alpha-zero RGB darkening from mip1 while OPAQUE
ignores alpha. Its original teal triangle measured63/158/150 at magnification,
then49/121/114 near LOD0.47, and33–40/33–49/32–46 in a farther oblique row around
LOD0.83–0.91. Those are historical evidence, not newly executed0043 measurements.
The all-alpha-zero case was only an inference in that review;0043 should include
it alongside mixed alpha. The previously shared generated-tangent sign defect is
already fixed in both frozen0043 binaries and is outside this delta.
