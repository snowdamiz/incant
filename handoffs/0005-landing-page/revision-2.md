# Revision 2: rationale

Director feedback: "the design needs to be heavily refined and improved. Right now it looks vibe coded and a bit generic."

The feedback is correct. Passing tests did not make the first version good.

## Critique of the first version

Based on the original renders in `screenshots/desktop-1440-full.png` and `screenshots/mobile-390-full.png`.

1. **Hero composition:** the hero followed the stock AI-product template. It had a centered stack, a pill badge, a two-tone gradient headline, two pill buttons, a purple radial glow and a dot grid. Nothing in it was specific to Incant, so it could have been any SaaS page.
2. **Colour:** violet glow was spent everywhere as atmosphere. Because the brand colour decorated everything, it could not mark anything.
3. **Repeated pattern:** every section used a tracked mono micro-label, a large sans heading and a grey paragraph. On top of that came six near-identical rounded cards, each with a miniature UI. The reading rhythm was flat.
4. **Typography:** Inter was the only voice at every size, so the page had no character beyond the default.
5. **Focal illustration:** the illustration was low-craft. Its UI type was 10 to 11px, the scene was clip-art built from gradients and primitives, and a glowing mascot was pasted on its corner. The workflow section then redrew the same scene smaller.
6. **Copy:** four sections restated the same idea, and the vocabulary was technical, with terms like `set_field`, "apply · invert · describe" and ULIDs.

## New direction: editorial studio

The page now reads like a creative tool's own publication rather than a template.

- **Ground and colour:** warm studio paper (`#f4f1ea`) and ink (`#17151d`), with hairline rules instead of cards and no glows, gradients-as-decoration or pills. The violet appears only where it means something: the active step, emphasis words, the wisp and one brand band.
- **Typography:** Fraunces, an OFL serif pinned at 5.3.0 using its "soft" axis, carries the display voice. Its rounded terminals echo the wisp's curves. Inter is used for reading and JetBrains Mono only inside the product illustration. The page has no decorative micro-labels.
- **Focal point:** the old hero image and workflow section are merged into one interactive editor stage. Four chapter tabs, Ask, Review, Play-test and Undo, sit directly above the editor. Each tab changes the scene, the outline, the inspector and the status bar together, so the core story becomes the first thing a visitor can use.
- **Illustration craft:** the level art is newly drawn. It has a cloaked wanderer on a cliff at dusk, layered ridges with atmospheric perspective, a floating isle with an anchor ring, a moon and a lighthouse.
  - In Review, the editor shows a selection box, a move gizmo and a dashed predicted path.
  - In Play-test, onion-skin frames trace the swing along that path.
  - Undo restores the scene.

  All UI text is real HTML at 12 to 13px or larger, so it stays legible on phones.
- **Structure:** the page went from seven blocks to five.
  1. The hero with its interactive stage.
  2. Four numbered principles set as an editorial list.
  3. A single violet statement band with a large paper-coloured wisp. It reads "Nothing the agent does is hidden. Nothing it does is permanent." Three trust facts sit under it.
  4. A five-question FAQ.
  5. A large closing line, "Start with a sentence.", followed by the footer.
- **Copy:** text is shorter and plain. The headline is "The game engine you can talk to."

## Kept from the first version

- The wisp redraw, now set flat in ink, paper or violet with no glow.
- The finished-product voice, with genuine repository destinations only.
- The skip link, the keyboard-accessible tabs and mobile menu with Escape, the native FAQ disclosure and reduced-motion handling.
