# Expand and polish the landing page around the full Incant plan

## Latest director correction — a middle ground for the workflow

The director has reviewed the graphical pass in the browser and supplied a
screenshot of the “One project, from shape to ship” workflow section. Their exact
feedback: “This section was taken to far in terms of removing text. It must find
a good middle ground”. This instruction takes precedence over the earlier push
to minimize all text. Implement this focused correction now.

The screenshot shows the six workflow icons and one-word labels, a large empty
band, and the platform logo row. It looks under-explained. Keep the graphical
pipeline and authentic platform SVGs, but restore useful, concise visible copy:
a short introduction if it helps and one brief explanation for each workflow
step, explaining what the visitor does or gets. Make those explanations concrete
and differentiated (modeling, materials, TypeScript logic, reviewing agent edits,
play-test evidence, export). Do not rely on screen-reader-only descriptions to
explain the visible workflow. Aim for an understandable middle ground between
bare icon labels and the original long feature paragraphs. Keep the overall page
graphical; do not expand every other section again.

Rebalance this section's spacing and hierarchy so the explanations belong to
their icons and the workflow and platform row feel related. The supplied
screenshot has excessive vertical dead space; retain comfortable breathing room
without preserving empty bands just to make the section large. Ensure the mobile
rail can accommodate the concise explanations naturally. Keep the actual SVG
platform icons and small platform labels the director requested.

Continue from your committed 81a7f9b. Astra has separately integrated it and added
license links plus a test selector fix; leave SiteFooter.vue and browser tests
alone so those integration changes are preserved. Scope this correction to the
workflow component/content and handoff evidence unless a closely related styling
change is necessary. Preserve the prior graphical result as history. Run strict
typecheck and production builds (including /incant/), commit with Built-by:
claude, and return the result for integration. No push, merge or publishing.

No computer use, browser/native automation, Playwright execution, screenshots,
recording or capture. The user's attached screenshot is feedback, not permission
to resume capture. Terminal source work only; hosted CI will validate behavior.
Do not claim an agent rendered review. Keep the neutral palette, restrained
accent, finished-product voice and other approved choices. Use judgment and
continue without asking routine design permission.

## Latest director instruction — redesign around graphics, not text

The director reviewed the density pass and says: “landing page it still too text
heavy. Needs more graphical things. As one example the platforms can be
represented as their actual svg icons instead of text. REdesign again with this
in mind”. Treat this as a substantial new design pass, not another copy trim.

Make the added sections communicate through graphics. Use authentic, recognizable
SVG platform marks instead of the current large platform-name word row. Use real
vector assets from official sources or a reputable icon library, retain any
required licensing/attribution, and keep accessible platform names. Do not replace
these with emoji, text glyphs or invented logos. Keep visible labels minimal;
choose the appropriate recognizable representation for Windows, macOS, Linux,
iOS, Android and Web while preserving their distinction for assistive technology.
Network/source downloads via terminal or non-browser fetch are permitted; browser
and computer-use/capture tools are still prohibited.

The platform row is one example, not the whole request. Rethink the tool overview
and workflow to rely on graphical composition, meaningful diagrams, mini product
illustrations or purposeful iconography. The interaction/model should tell the
story with minimal supporting copy. Reduce the repeated term-and-description
lists and decorative large words. Use the page's established visual language;
do not make a generic icon-card grid or add token graphics next to the same wall
of text. Keep useful product detail available without forcing a visitor to read
every capability. Preserve the core modeling/game-engine/agent promise and the
existing working walkthroughs. Give the page room and a clear visual hierarchy.

Preserve the finished-product voice (no future labels), neutral paper/ink and
charcoal palette, restrained accent, approved wisp and connected product panels.
No computer use, browser/native automation, Playwright execution or captures on
this machine. Work through source and nonvisual terminal validation only; hosted
CI will handle browser tests after integration. Do not claim rendered review.

Continue from 80e3649, integrated on the landing branch as 1acb2d5. Its 23 hosted
browser tests passed, but that does not establish design acceptance. Preserve
prior result packets, produce the new design, run strict typechecking and both
production builds, and commit with Built-by: claude. Include sources/licenses for
SVG assets, the concrete graphical changes, checks and remaining limitations.
Scope remains website/ and this handoff. No publishing, pushing or merging.

The director also said: “next time dont ask, just continue if it makes sense to
do so”. Use judgment for routine authorized design and implementation choices.
Do not ask repeated permission; preserve the explicit screen-capture restriction.

## Latest director feedback — the expanded page is too dense

The director has now viewed the new page and says: “some of the new sections are
way to text heavy and close together overall, not good enough”. Rework the new
sections and overall page pacing now. This is a request for a stronger design
pass, not just another paragraph or a small spacing adjustment.

Cut the amount of visible copy substantially. Give the modeling and workflow
sections room to breathe, reduce competing headings/labels and repeated prose,
and let the modeling illustration and a few strong ideas carry the explanation.
The six tool descriptions plus the six workflow descriptions currently read too
much like documentation. Do not turn PLAN.md into a feature inventory. Preserve
its important product promise through concise, well-chosen content and an
intentional visual hierarchy. Decide where short summaries, visual groupings or
useful progressive disclosure improve the page. Avoid hiding everything merely
to reduce length, and avoid replacing the prose wall with generic card grids.

Review the whole page's rhythm: transitions into and out of the two new sections,
the relationship between heading/description/illustration, whitespace within and
between groups, column density and mobile stacking. Improve the design as a
cohesive page, not isolated blocks. Keep the interactive geometry illustration,
but refine its role and surrounding composition as needed. Preserve keyboard and
screen-reader behavior and keep the existing agent walkthrough working.

The director's finished-product presentation is still required: do not restore
future/planned/early-development labels. Keep the approved neutral palette,
restrained accent, wisp and connected-panel product chrome. The screen-capture
restriction remains in force: no computer use, browsers, Playwright execution,
native automation or capture tools. Source edits and nonvisual terminal checks
only. Hosted CI will run browser tests after Astra integrates the result.

The prior revision 317cf6b is integrated on the landing branch as 11abac8 and is
available in the user's existing preview. Continue from the files already here.
Preserve previous result evidence as history, append or write a new result for
this design pass, run strict type checking and the /incant/ build, and commit with
Built-by: claude for integration. Do not publish, merge or push. Explain the
concrete changes in hierarchy, copy density and spacing, with actual checks and
remaining verification limits. Do not claim you saw a render.

## Director correction — finished-product presentation

The director just clarified: “no just tell claude to design it as if it all
already shipped because it will soon. this lableing as future is useless extra
step”. Follow this instruction now. Present the complete PLAN.md product in the
present tense, as a finished product. Remove “planned”, “future”, “early
development”, “not yet” and similar qualification/badges from public marketing
copy. Do not add a current-versus-roadmap section. This supersedes the earlier
Astra request to qualify unshipped capabilities; that extra step was not wanted.
Keep building and polishing the expanded modeling/workflow design already in
progress. Preserve your existing file changes and inspect them when resuming.
Internal handoff/evidence can still state actual verification and implementation
scope; public product presentation does not approve an engine phase gate.
The screen-capture prohibition remains in force without exception.

Also correct two factual details during this pass: the repository is now public
(verified through GitHub), so remove stale “private repository”/“may require
access” copy. The director changed the macOS credential store since this worktree's
PLAN.md copy: macOS uses private owner-only local files; Windows/Linux use native
credential stores. Public copy can simply say credentials stay on the user's
machine rather than promise a keychain universally. Do not edit PLAN.md or auth.

## Latest director request — implement this follow-up now

“tell the landing page claude session to add more to the landing page and continue
polishing the design. It should go based off of what the plan document promises.
As one example, its missing the 3d modeling capabailities it will have”

This supersedes the previous notification-only stop after the palette revision.
Resume this same Claude Opus 5.5 ACP session and implement an expanded, polished
landing page. The screen-capture restriction itself REMAINS IN FORCE.

## Mandatory current restriction

The director is watching Prime Video, which screen capture interferes with.
Do not use computer-use tools, browser or native automation, Playwright execution,
screenshots, screen recording, screen sharing, or capture helpers. Do not launch
any browser or native review, even headless. Continue through source/code/file
work and nonvisual terminal checks only. Existing images may stay on disk. Do not
interpret this new request for design work as permission to resume capture.
The runner's generic screenshot request is superseded by this restriction.
Record the resulting visual-verification limitation clearly in result.md.

## Product and content direction

Read the complete PLAN.md in this worktree, especially section 1, the phase
deliverables, sections 3 and 6, and the product's stated definition of done.
Use the plan to decide what meaningful product capabilities the page currently
omits. The page should communicate a modeling tool, game engine and editor with
an integrated agent, not only an AI chat/edit loop.

Give planned 3D modeling/content creation real prominence and a substantive,
carefully designed section or sequence. Phase 4 explicitly includes:
- Procedural geometry graphs: primitives, booleans, extrude, bevel, subdivision,
  arrays, scatter, curves, lofts, instancing and noise displacement.
- Mesh cleanup, automatic retopology and UVs, LODs, decimation, normal/AO baking,
  and limited brush sculpting for adjustments.
- Shader/material graphs, terrain editing and layered materials/foliage,
  animation retargeting, IK, animation graphs and cinematic timelines, and VFX.
- Pluggable generated assets, including image-to-3D with cleanup, texture sets
  and animation, alongside conventional imported assets.

Choose a coherent presentation; do not dump this list verbatim into cards.
Consider a procedural modeling/product illustration and a concrete workflow that
connects modeling, materials, game logic, agent review, play-testing and export.
Represent only supported plan promises; full Blender sculpting, Nanite and visual
scripting are explicitly out of scope. Do not imply those capabilities exist.

Also audit the broader promise: 2D/3D creation, TypeScript gameplay, shared
reversible edits with review/history, offline editing, users' own provider
connections, collaboration and the six export targets. Emphasize what helps a
visitor understand Incant. Keep content readable, paced and distinct, with useful
navigation and honest calls to action. Use the finished-product presentation requested above for the full feature set.
Keep real calls to action; do not invent adoption statistics, launch dates,
pricing, testimonials or nonexistent download links.

## Design direction

Continue polishing the accepted editorial layout, connected editor panels,
typography, responsive behavior, spacing and hierarchy. Preserve the approved
wisp artwork, neutral paper/ink page palette, charcoal/gray editor depiction and
restrained accent from 6090702. The director explicitly rejected excessive
purple and the earlier spaced-out card/panel treatment. New illustrations and
sections should feel intentional and part of the same product, not bolted on.
Write any new artwork as lightweight local SVG/CSS where appropriate; keep
accessibility, keyboard behavior, reduced motion and small-screen layout sound.
Avoid new runtime dependencies unless there is a concrete need. Keep the page
fast and retain the production /incant/ base-path behavior.

## Scope, verification and handoff

Work only in website/ and handoffs/0007-landing-neutral/. No native editor, engine,
auth, PLAN.md, deployment/account/settings, credential, or unrelated changes.
No publishing, merging or pushing. Preserve existing work and the previous result
as history (append a clearly dated follow-up or save a separate prior result).

Run terminal-only type checking and production builds. Update meaningful browser
behavior/accessibility tests where new navigation or interaction warrants it, but
do not execute browser tests locally under the current restriction. Astra will
integrate and let the existing hosted CI run those tests remotely. Do not add
brittle tests that just mirror copy, CSS tokens or implementation details.

Return a result identifying the exact model, changes, commands/results, deferred
visual checks and remaining limitations. Include a concise promise-to-page map
with PLAN.md section references so Astra can audit factual accuracy. Commit the
finished code and packet with Built-by: claude for integration into existing PR #2.
Open permissions were explicitly authorized by the director. Do not substitute
another model or request repeated permission for routine authorized file work.
