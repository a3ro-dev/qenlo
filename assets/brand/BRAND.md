# Qenlo Brand Guidelines

This guide establishes the visual identity, typography standards, design tokens, and asset usage rules for **Qenlo**.

---

## 1. Product Truth &amp; Positioning

### What Qenlo Is
**Qenlo is a Rust vector-database research project for native applications.**

Qenlo is designed for developers embedding dense vector search directly into client applications. The primary design target is desktop environments first, with mobile and web browser targets planned for subsequent research phases.

### Factual Copy &amp; Technical Scope
- **Current Milestone**: An early research prototype testing a vector-search hypothesis.
- **Scope of Engine**: Indexes and performs approximate nearest-neighbor (ANN) search over existing, pre-computed vector embeddings.
- **External Dependencies**: Embedding model generation, tensor transformations, and NPU/neural inference belong strictly to external runtimes.
- **Research Objectives**: Hardware-assisted acceleration (e.g. SIMD/GPU) and improved filtered-query latency are exploratory research objectives, not shipping capabilities.
- **Integrity Rule**: Never claim production readiness, universal platform compatibility, a finalized public API, benchmark superiority, customers, or endorsements. Never invent benchmark numbers.

### Naming &amp; Voice
- **Human-Facing Product Name**: `Qenlo` (Title case).
- **Package / Identifier Namespace**: `qenlo` (lowercase, e.g. `qenlo-core`, `cargo add qenlo`).
- **Brand Voice**: Editorial, research-instrument precision, readable, concise sentence-case copy.

---

## 2. Visual Identity &amp; Geometry

The Qenlo emblem simplifies the project's original hexagonal coordinate lattice into clean mathematical vector geometry without ornamental raster debris.

```
                  ┌─┬─┐
                  │ │ │  (North 3-Prong Terminal)
                  └─┼─┘
                 ▲  │
               /   \│
             /      ▼ (Hexagonal Boundary)
           /   ┌─┐    \
   ───┬───┤    │ │     ├───┬─── (West & East Cardinal Terminals)
      │   │    └─┘ ─── │   │
           \      ▲   /
             \    │  /
               \  │ /
                  │
                  ▼
                └─┼─┘
                │ │ │  (South 3-Prong Terminal)
                └─┴─┘
```

- **Hexagonal Lattice**: Represents the multi-dimensional metric space and coordinate neighborhood.
- **Cardinal Bus Terminals**: 3-prong connectors positioned at the 4 cardinal directions (North, South, East, West) representing low-level systems integration and native I/O.
- **Central Core &amp; Radiating Axes**: Solid hexagonal hub with 4-axis isometric vector basis vectors.
- **Discrete Red Node**: Positioned at coordinate `(334, 301)` representing a target vector coordinate in the index.
- **Single-Color Integrity**: The mark is structurally distinct in 100% monochrome. The red accent reinforces the identity but is not solely relied upon for recognition.

---

## 3. Color Palette &amp; Design Tokens

All color tokens are namespaced under `--qenlo-color-*` and defined in `tokens.json` and `tokens.css`.

### Light Palette (Canvas Default)
| Token | Hex Value | Role / Usage | Minimum Contrast |
|---|---|---|---|
| `--qenlo-color-background` | `#F7F5F0` | Canvas base surface | — |
| `--qenlo-color-text` | `#1D2320` | Primary headings, body copy, structural strokes | 14.67:1 (AAA) |
| `--qenlo-color-muted-text` | `#5A615D` | Secondary captions, metadata, subtle borders | 5.84:1 (AA) |
| `--qenlo-color-accent` | `#B53C2F` | Primary brand accent, discrete vector coordinate node | 5.28:1 (AA) |
| `--qenlo-color-on-accent` | `#FFFFFF` | Text/icons on accent backgrounds | 5.75:1 (AA) |
| `--qenlo-color-surface` | `#EFECE6` | Panels, card containers, table backgrounds | 1.05:1 vs canvas |
| `--qenlo-color-surface-raised` | `#E6E2DA` | Elevated code blocks, specimen cards | 1.15:1 vs canvas |
| `--qenlo-color-border` | `#D8D3C8` | Subtle structural dividers and rules | Decorative |
| `--qenlo-color-border-strong` | `#878E8A` | High-contrast container borders, active inputs | 3.12:1 (UI 3:1) |
| `--qenlo-color-focus` | `#B53C2F` | Accessible focus ring outline | 5.28:1 (AA) |

### Dark Palette (Reversed / Dark Canvas)
| Token | Hex Value | Role / Usage | Minimum Contrast |
|---|---|---|---|
| `--qenlo-color-background` | `#171C19` | Dark canvas base surface | — |
| `--qenlo-color-text` | `#F7F5F0` | Primary text and structural mark outlines | 15.84:1 (AAA) |
| `--qenlo-color-muted-text` | `#AAB3AD` | Secondary captions, metadata, lattice rules | 8.02:1 (AAA) |
| `--qenlo-color-accent` | `#EF8B79` | Bright coral accent node and focus ring | 7.10:1 (AAA) |
| `--qenlo-color-on-accent` | `#171C19` | Text/icons on coral accent fill | 7.10:1 (AAA) |
| `--qenlo-color-surface` | `#1F2622` | Dark panel containers, cards | 1.10:1 vs canvas |
| `--qenlo-color-surface-raised` | `#2A332E` | Dark code blocks, elevated surfaces | 1.25:1 vs canvas |
| `--qenlo-color-border` | `#2E3833` | Subtle structural dividers | Decorative |
| `--qenlo-color-border-strong` | `#6E7C74` | Active borders and UI boundaries | 3.98:1 (UI 3:1) |
| `--qenlo-color-focus` | `#EF8B79` | Accessible focus ring outline | 7.10:1 (AAA) |

---

## 4. Typography

Brand typography is built around the **IBM Plex** family, bundled locally in `fonts/` for offline use.

```
Prose & Headings: IBM Plex Sans (Regular 400, SemiBold 600)
Code & Measurements: IBM Plex Mono (Regular 400, Tabular Figures)
```

- **IBM Plex Sans Regular (400)**: Used for body text, documentation prose, and descriptions.
- **IBM Plex Sans SemiBold (600)**: Used for headings, section titles, button labels, and wordmarks.
- **IBM Plex Mono Regular (400)**: Used for code snippets, configuration keys, memory measurements, dimension counts, and technical badges.
- **Tabular Figures**: Always enable `font-variant-numeric: tabular-nums` when displaying latency, dimensions, or benchmark matrices.
- **Outlined Logo Letterforms**: All logo SVGs have outlined vector bezier paths so they render identically without installing fonts.
- **Platform UI Exception**: Future native desktop/mobile application UI controls may use native platform fonts (e.g. Segoe UI, San Francisco) to respect OS ergonomics.

---

## 5. Asset Catalog &amp; Usage Rules

All vector assets are self-contained and located in `assets/brand/`:

```
assets/brand/
├── logo/
│   ├── mark.svg             # Standalone emblem (Light surfaces)
│   ├── mark-reversed.svg    # Standalone emblem (Dark surfaces)
│   ├── lockup.svg           # Horizontal emblem + wordmark (Light surfaces)
│   ├── lockup-reversed.svg  # Horizontal emblem + wordmark (Dark surfaces)
│   └── favicon.svg          # Simplified 32x32/16x16 micro-scale icon
└── social/
    ├── card.svg             # Editable 1200x630 social card
    └── card.png             # 1200x630 rendered social preview image
```

### Variant Selection Matrix
| Asset | Primary Use Case | Minimum Size |
|---|---|---|
| `mark.svg` | App headers, avatar icons, diagrams on light backgrounds | `32 × 32 px` |
| `mark-reversed.svg` | App headers, terminal banners on dark backgrounds | `32 × 32 px` |
| `lockup.svg` | Documentation headers, website headers (light theme) | `24 px` height |
| `lockup-reversed.svg` | Documentation headers, terminal readmes (dark theme) | `24 px` height |
| `favicon.svg` | Browser tab favicons, system tray, bookmark icons | `16 × 16 px` |
| `card.svg` / `card.png` | GitHub social preview, technical article cards | `1200 × 630 px` |

### Clear Space Rules
Maintain a clear space around the emblem and lockup equal to **at least half the height of the mark (`0.5H`)** on all four sides. No text, rules, or background distractions should encroach into this exclusion zone.

```
       ┌───────────────────────────────┐
       │             0.5H              │
       │     ┌───────────────────┐     │
  0.5H │ 0.5H│  [Emblem]  Qenlo  │0.5H │ 0.5H
       │     └───────────────────┘     │
       │             0.5H              │
       └───────────────────────────────┘
```

---

## 6. Logo Misuse (What NOT to Do)

1. **Do not distort or skew**: Never alter the aspect ratio or stretch the mark horizontally or vertically.
2. **Do not replace the emblem**: Do not replace the emblem with a generic letter "Q", database cylinder, glowing chip, or neural net icon.
3. **Do not add decorative effects**: Never add drop shadows, outer glow, glossy reflections, 3D extrusions, or gradients.
4. **Do not use unauthorized colors**: Only use the approved token palette (`#B53C2F` / `#EF8B79` for accents).
5. **Do not invert colors mechanically**: Always use `mark-reversed.svg` on dark backgrounds instead of applying CSS `filter: invert()`.
6. **Do not crowd the mark**: Never violate the `0.5H` clear space perimeter.
7. **Do not alter wordmark typography**: Do not re-type the wordmark in an unapproved typeface or modify character kerning.

---

## 7. Upstream Provenance &amp; Licensing

### Font Deliverables
- **Upstream Repository**: [IBM/plex (GitHub)](https://github.com/IBM/plex)
- **IBM Plex Sans**: Release package `@ibm/plex-sans@1.1.0`
  - `IBMPlexSans-Regular.woff2` (SHA-256: `ba711a3085ff9f27440b6b9c4550cfc47c97bf36591d5da958b975bb3add8c1a`)
  - `IBMPlexSans-SemiBold.woff2` (SHA-256: `f78048030eab62e860efa39a0df79e2e5581bf122eb95b9bc42c0b8a4988d205`)
- **IBM Plex Mono**: Release package `@ibm/plex-mono@2.5.0`
  - `IBMPlexMono-Regular.woff2` (SHA-256: `ba204497f16b6d334cee9d1e963a831b73e3a56e1d6300a8489d18df7214b350`)
- **License**: SIL Open Font License (OFL) Version 1.1 (included at `fonts/OFL.txt`).
