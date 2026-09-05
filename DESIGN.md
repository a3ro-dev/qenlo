# Qenlo interface system

Qenlo’s browser is a precise local instrument, not a generic administration dashboard. The interface should make collection state, filtering, search execution, and durability legible at a glance.

## Register and scene

Product interface. A developer uses it on a laptop or external monitor while debugging a local retrieval pipeline, often beside an editor and terminal. Dark mode is primary to coexist with that workspace; light mode remains fully supported.

## Visual language

- Warm graphite surfaces with a quiet mineral tint.
- Persimmon is reserved for selection, focus, and primary actions.
- Mint, amber, and red communicate healthy, caution, and destructive states with accompanying text or symbols.
- System sans for controls and prose; system monospace for vectors, identifiers, paths, timings, and generations.
- Corners are compact rather than pill-shaped. Borders establish data regions; shadows are limited to overlays.
- Dense tables and split views are preferred over repeated cards.

## Spacing and type

- Spacing follows a 4 px base: 4, 8, 12, 16, 24, 32.
- Body text is 14 px on desktop, with 12 px labels and 18–20 px view titles.
- Controls have a minimum 36 px desktop height and a visible 2 px focus ring.
- Numeric data uses tabular figures.

## Motion

- Frequent and keyboard-driven interactions are immediate.
- Pointer-triggered state changes use 150–220 ms transitions with `cubic-bezier(0.16, 1, 0.3, 1)`.
- Overlays enter with opacity, 6 px translation, and slight blur. Their exit is shorter and quieter.
- Animate only opacity and transforms. Respect `prefers-reduced-motion`.

## Components

- Workspace rail: stable navigation and collection identity.
- Context bar: path, dimensions, live rows, generation, and readiness.
- Work surface: one dominant task area without nested cards.
- Status chips: icon or symbol plus text, never color alone.
- Empty states: explain why the area is empty and name the next useful action.

## Avoid

- Emoji as interface icons.
- Decorative gradients, glass panels, neon glows, and generic metric-card grids.
- Accent color on inactive controls.
- Hidden failures, ambiguous backend status, or performance claims without context.
