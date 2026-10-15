# Inventory

## Workflow

1. Query the existing item catalog and scripts; define stable item IDs, maximum slots, stack limits and explicit pickup/drop/equip rules. Store configuration in Script props or supported typed assets, and bounded mutable counts/equipment in versioned context.state. Do not invent an Inventory engine component.
2. Process accepted pickup interactions once. Validate item IDs, capacity, counts and ownership before changing state. A rejected full-inventory pickup must leave both the inventory and world item intact; successful consumption must not duplicate on the next tick.
3. Render labels and counts through localized UI controls. Queue supported world changes through context.commands. If the runtime cannot instantiate the required item archetype, use an explicitly bounded authored pool or report that limitation; do not pretend Transform-only spawn creates a complete asset.
4. Define save migration and reset rules, including equipped items and pending interactions. Keep authored item definitions out of mutable runtime counters.

## Verification

Test pickup, duplicate events, stack overflow, full inventory, equip/unequip, drop, unknown items and restart/save restoration. Assert conservation of item counts across each transaction and no negative or nonfinite quantities. Verify denied operations leave both UI and world unchanged.
