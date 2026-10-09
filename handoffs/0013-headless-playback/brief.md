# Headless simulated frames: rendered-pixel review

Claude Opus 5.5 via ACP owns this focused pixel review. Astra implemented bounded
headless playback, reusing the current diagnostic renderer and simulation command
bus. No layout, shader appearance, camera or native UI changes are in this packet.
Review the ignored artifacts/playback-review/velocity and /scripted PNG sequences.
They are actual Apple M5 Pro GPU readbacks, not mocked screenshots.

Each sequence has frame-000000.png, frame-000030.png and frame-000060.png, 640x360.
Two imported instances share one indexed draw. Original sources were removed;
only the cooked cache was used. The velocity sequence moves both instances left
by two world units over 60 ticks. The scripted sequence additionally uses the
bundled kinematic TypeScript template to move right one unit per second, for a
net one-unit leftward motion. Both start at the same geometry/positions. Review
that motion, frame coherence, visibility and preservation of diagnostic appearance.
Do not claim production lighting/materials, camera controls or performance gates.

Return handoffs/0013-headless-playback/result.md with exact model and ACP transport,
inspected frames, verdict and limitations. This is review-only unless you find a
specific appearance regression; report correctness findings to Astra. Do not
change code, dependencies, fixtures or generated artifacts. No tests/builds are
needed for review-only documentation. Do not access credentials or accounts,
publish, merge, capture native UI, or edit outside this worktree. Commit the review
with Built-by: claude. Numeric hashes alone do not substitute for pixel review.
