# Align landing-page colors with the director’s preferred editor palette

## Latest director instruction

“tell claude its a bit too purple. I liked the previous designes color choices
better. Update the app and landing page to match this change. Also logo is too
closeto traffic lights and not aligned”.

You are Claude Opus 5.5 through ACP. The director has authorized open permissions
for this work. A separate Claude packet owns the actual editor and its native
traffic-light/logo correction; your scope is the landing page only. Use the same
shared color reference, references/previous-editor-tokens.css from the earlier
neutral editor design, so the product depiction matches the app.

The completed landing-page revision 2 is already in this worktree under website/.
Read its handoff 0005 rationale and screenshots, and inspect the current rendered
page. Preserve the layout the director liked, especially the connected editor
panels and revised editorial structure. Reduce purple across the page and its
editor illustration, bringing the product chrome back to neutral charcoal/gray.
Keep the page coherent and the approved wisp artwork intact. Violet should be a
restrained accent rather than a tint across backgrounds and controls. Do not
restore the superseded card-heavy landing page or lose working interactions.

Edit website/ and this packet only. No engine/native/auth files, external accounts,
deployment changes, repository settings, signing, merging or publishing. Do not
change or inspect credentials. Preserve existing user work. Any cross-platform
color decision should be written to palette-notes.md for Astra to share with the
editor packet if needed. Do not invoke native UI automation or override HOME.

Use the existing Vue/Vite browser test workflow and production /incant/ base path.
Run a fresh build and browser behavioral/accessibility checks; inspect and refine
actual desktop and mobile renders. Check Ask, Review, Play-test, Undo, navigation,
menu, focus, contrast and reduced-motion states. Keep before/after screenshots,
with exact commands/results, model and limitations in result.md. Commit final
website changes and evidence with Built-by: claude for Astra to integrate into the
existing landing PR. Do not approve a phase gate or claim deployment happened.
