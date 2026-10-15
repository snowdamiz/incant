# Day/night cycle

## Workflow

1. Query authored lighting, sky/environment controls and the native API that can change them at runtime. Define a period, initial phase and explicit limits for intensity/color/elevation. Static authoring edits alone do not implement a runtime cycle.
2. Advance phase from accepted fixed simulation ticks, with deterministic wrapping and pause/restore behavior. Interpolate authored keyframes in a declared color space and avoid discontinuities at the wrap.
3. Update only supported runtime lighting controls, at a justified bounded frequency. Do not rebuild the document, recook assets or create pipelines every frame. If the host cannot mutate a required light/environment parameter, report that implementation gap explicitly.
4. Persist phase and configuration identity through supported script state/save contracts. Keep gameplay time, real-world time and locale formatting separate.

## Verification

Check keyframe values, monotonic phase, wrap continuity, pause/restart/restore and very short/long periods. Review shadows and transitions in the actual renderer when permitted. Measure the complete rendering cost before claiming the cycle is inexpensive.
