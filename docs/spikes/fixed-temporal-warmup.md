# Fixed temporal history reset ramp

Claude's `0072` sharpness comparison identified a registration regression after
history reset: the initial registered edge sample's bias decayed too slowly with
a constant 0.9 history weight. Its retained GPU result was 0.207 px at frame six,
against the proposed 0.1 px threshold. The accepted coverage baseline and the
rejected sharpness candidate remain distinct; this change is experimental until
its actual GPU comparison is reviewed.

The host now supplies `min(configured_weight, (samples - 1) / samples)` through
the existing uniform. A new history starts at one accepted sample; bypass has
zero. Age advances only when the frame is accepted, saturates at 100 (enough for
the largest allowed 0.99 weight), and restarts with the existing motion-owner
reset. Preparation and failed submissions cannot publish age. Shader source,
bindings, resource limits and the configured steady-state weight are unchanged.

Three CPU tests check reset/bypass behavior, the independently predicted mean of
an initially biased edge, and policy bounds over a long session. Assertions in
the existing retained GPU test check age across failed candidates, submission,
owner replacement and resolution reset without adding any render submissions.
Those GPU assertions have not yet executed for this candidate.

The source-frozen local qualification passes 133 default and 141 cooked-material
CPU tests; 41 GPU tests remain ignored in each run. Strict cooked-material
all-target Clippy, formatting, generated conventions and diff checks pass.
The [qualification archive](evidence/fixed-temporal-warmup.json) records exact
commands, source identities and lossless logs. No phone/performance or appearance
gate is claimed. The previous sharpness comparison's 13 quality failures remain
open, including flicker, trailing edges and the separate viewport-shadow issue.
