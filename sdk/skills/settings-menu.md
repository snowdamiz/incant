# Settings menu

## Workflow

1. Inventory supported audio, input, display, locale and accessibility settings. Separate runtime settings from authored defaults and persistent user preferences. Never offer a setting whose host capability is absent as if it were functional.
2. Build localized labels and native UI controls with keyboard/gamepad focus order. Keep an editable draft separate from applied values. Validate ranges and compatibility before applying; define reset-to-defaults and cancel semantics.
3. Apply each setting through its actual runtime/host API, preserving unrelated values. Destructive display changes need a reversible confirmation timeout only when the host supports recovery. A game script cannot write an arbitrary preferences path.
4. Persist preferences only through an available trusted host mechanism, with versioning and fallback for invalid old values. Without persistence, state clearly that the settings are session-only.

## Verification

Test apply, cancel, defaults, repeated menu opening, invalid saved values and unsupported hardware. Check audio/locale effects with actual observations, not just slider labels. Confirm gameplay input is released while a text/control interaction owns focus and restored when the menu closes.
