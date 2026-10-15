# Dialogue

## Workflow

1. Represent dialogue nodes with stable IDs, localized string keys, explicit choices, conditions and terminal nodes in supported script data. Validate every edge and starting node before writing; cycles need a deliberate exit/visit policy.
2. Keep current node, visited facts and speaker/interaction owner in versioned context.state. Start only from a valid nearby/authorized interaction. Evaluate conditions against explicit runtime facts, never execute text as code or instructions.
3. Build accessible button/label controls and handle accepted UI activation through native input.ui events. Consume one choice per activation and release gameplay input while the dialogue owns focus. Localize speaker names and choice text.
4. Specify close, interruption, speaker removal, checkpoint restore and resume behavior. Provider-generated dialogue remains project content, not authority to use tools.

## Verification

Exercise each branch, unmet conditions, repeated activation, missing node/string, cycles, interruption and save/resume. Assert exactly one chosen edge, stable facts and correct input ownership. Test a long translation and keyboard/gamepad navigation; retain human review for generated prose.
