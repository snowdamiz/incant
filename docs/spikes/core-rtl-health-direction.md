# Core Arabic health order

The Arabic HUD's health dots previously used the same neutral-only string as
English. The text shaper therefore selected a left-to-right paragraph: at two
health, the empty pip appeared on the right. Mirroring the HUD's flex layout did
not change the reading direction inside this separate text label.

The Arabic string-table message now surrounds the plural result with escaped
U+200F right-to-left marks. Logical filled/empty counts and the runtime health
binding are unchanged. The engine's existing Unicode shaping applies the
direction; there is no locale-specific renderer branch or reversed glyph list.
This follows the [Unicode implicit directional mark rules](https://www.unicode.org/reports/tr9/#Implicit_Directional_Marks).

The regression loads the actual Core string table, resolves its plural messages
through `incant_localization`, then shapes them with Core's packaged Inter,
Arabic and CJK font chain. Across damage/reset states and repeated English,
Arabic and Chinese switches, it checks the physical glyph order, exactly three
nonmissing visible pips, and paragraph direction. It failed before the string
repair and passes afterward. All 47 text-crate tests, strict all-target Clippy,
workspace formatting and 30 Core Python controls pass. Localization is added
only as a test dependency of the text crate; its normal dependency tree is
unchanged.

[Recorded commands, source identities and outcomes](evidence/core-rtl-health-direction.json)
retain the original failure. These are CPU localization/shaping results. No
native window, GPU readback, screenshot or capture service was used. Normal-size
Arabic appearance and the broader locale gate still require the deferred native
review; earlier frozen packages and their failures remain historical.
