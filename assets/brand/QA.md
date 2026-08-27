# Quality Assurance &amp; Verification Report

This document records the verification procedures, automated checks, contrast calculations, and quality standards executed on the **Qenlo** brandkit deliverables.

---

## 1. Scope of Deliverables Verified

| File Path | Description | Status |
|---|---|---|
| `assets/brand/BRAND.md` | Brand guidelines and factual copy rules | Verified |
| `assets/brand/tokens.json` | Canonical design system tokens | Verified |
| `assets/brand/tokens.css` | CSS custom properties with 100% token parity | Verified |
| `assets/brand/preview.html` | Offline brand specimen & review sheet | Verified |
| `assets/brand/QA.md` | QA report and verification records | Verified |
| `assets/brand/fonts/IBMPlexSans-Regular.woff2` | Upstream bundled font (Regular 400) | Verified |
| `assets/brand/fonts/IBMPlexSans-SemiBold.woff2` | Upstream bundled font (SemiBold 600) | Verified |
| `assets/brand/fonts/IBMPlexMono-Regular.woff2` | Upstream bundled font (Mono 400) | Verified |
| `assets/brand/fonts/OFL.txt` | SIL Open Font License 1.1 | Verified |
| `assets/brand/logo/mark.svg` | Standalone emblem (Light surfaces) | Verified |
| `assets/brand/logo/mark-reversed.svg` | Standalone emblem (Dark surfaces) | Verified |
| `assets/brand/logo/lockup.svg` | Horizontal lockup with outlined letterforms | Verified |
| `assets/brand/logo/lockup-reversed.svg` | Dark horizontal lockup with outlined letterforms | Verified |
| `assets/brand/logo/favicon.svg` | 32×32 / 16×16 micro-scale icon | Verified |
| `assets/brand/social/card.svg` | 1200×630 editable social preview card | Verified |
| `assets/brand/social/card.png` | 1200×630 rendered social preview image | Verified |

---

## 2. Vector SVG Asset Audit

Each SVG file was parsed and analyzed using Python XML parser (`xml.etree.ElementTree`):

| Asset | `viewBox` | XML Valid | Scripts (`<script>`) | Raster Images (`<image>`) | External Font Refs | Text Elements | Outlined Paths |
|---|---|---|---|---|---|---|---|
| `mark.svg` | `0 0 512 512` | Yes | 0 (None) | 0 (None) | 0 (None) | 0 | 100% Pure Vectors |
| `mark-reversed.svg` | `0 0 512 512` | Yes | 0 (None) | 0 (None) | 0 (None) | 0 | 100% Pure Vectors |
| `lockup.svg` | `0 0 460 120` | Yes | 0 (None) | 0 (None) | 0 (None) | 0 | 100% Outlined Beziers |
| `lockup-reversed.svg` | `0 0 460 120` | Yes | 0 (None) | 0 (None) | 0 (None) | 0 | 100% Outlined Beziers |
| `favicon.svg` | `0 0 32 32` | Yes | 0 (None) | 0 (None) | 0 (None) | 0 | 100% Pure Vectors |
| `card.svg` | `0 0 1200 630` | Yes | 0 (None) | 0 (None) | 0 (None) | 11 (Editable) | Hybrid Layout |

### Geometry &amp; Scale Inspection
- **16px Micro-Scale Recognition**: Tested `favicon.svg` at `16 × 16 px`. Thickened structural strokes (`2.2px` in 32px canvas), high-contrast cardinal nodes, and enlarged red accent point remain crisp and identifiable.
- **Monochrome Validation**: Rendered mark in 1-color (pure black on white and pure white on dark slate). The hexagonal lattice, 4 cardinal bus terminals, central core, and coordinate junction remain recognizable without color information.
- **Geometric Cleanliness**: Zero auto-tracing artifacts, zero stray nodes, zero overlapping duplicate strokes, and zero sub-pixel rounding jitter.

---

## 3. Font Asset Integrity &amp; Licensing

Font binaries were extracted from official upstream distribution repositories and verified using `fontTools.ttLib.TTFont`:

| Font File | Release Package | File Size | SHA-256 Checksum | Units Per EM | Glyphs Count |
|---|---|---|---|---|---|
| `IBMPlexSans-Regular.woff2` | `@ibm/plex-sans@1.1.0` | 63,020 B | `ba711a3085ff9f27440b6b9c4550cfc47c97bf36591d5da958b975bb3add8c1a` | 1000 | 1,019 |
| `IBMPlexSans-SemiBold.woff2` | `@ibm/plex-sans@1.1.0` | 67,060 B | `f78048030eab62e860efa39a0df79e2e5581bf122eb95b9bc42c0b8a4988d205` | 1000 | 1,019 |
| `IBMPlexMono-Regular.woff2` | `@ibm/plex-mono@2.5.0` | 49,248 B | `ba204497f16b6d334cee9d1e963a831b73e3a56e1d6300a8489d18df7214b350` | 1000 | 1,207 |

- **License**: SIL Open Font License 1.1 bundled at `fonts/OFL.txt`.
- **Offline Compliance**: Verified that `preview.html` loads all fonts strictly from local paths without external web requests or Google Fonts CDNs.

---

## 4. Token Parity Audit

Synchronized between `tokens.json` and `tokens.css`:
- **Total Token Definitions**: 29 tokens across Color, Typography, and Spacing scales.
- **Namespace**: `qenlo-` (`--qenlo-color-*`, `--qenlo-font-*`, `--qenlo-space-*`).
- **Parity Score**: 100% (0 mismatches).

---

## 5. WCAG 2.1 Contrast Calculation Matrix

Calculated using relative luminance formula $L = 0.2126 R_L + 0.7152 G_L + 0.0722 B_L$ and contrast ratio $(L_1 + 0.05) / (L_2 + 0.05)$:

| Color Pair | Foreground Hex | Background Hex | Contrast Ratio | WCAG 2.1 Threshold | Compliance Level |
|---|---|---|---|---|---|
| Light Primary Text | `#1D2320` | `#F7F5F0` | **14.67:1** | $\ge 4.5:1$ (Normal) | **Pass (AAA)** |
| Light Muted Text | `#5A615D` | `#F7F5F0` | **5.84:1** | $\ge 4.5:1$ (Normal) | **Pass (AA)** |
| Light Accent | `#B53C2F` | `#F7F5F0` | **5.28:1** | $\ge 4.5:1$ (Normal) | **Pass (AA)** |
| Light On-Accent Text | `#FFFFFF` | `#B53C2F` | **5.75:1** | $\ge 4.5:1$ (Normal) | **Pass (AA)** |
| Light Strong Border | `#878E8A` | `#F7F5F0` | **3.08:1** | $\ge 3.0:1$ (UI / Boundary) | **Pass (AA)** |
| Dark Primary Text | `#F7F5F0` | `#171C19` | **15.84:1** | $\ge 4.5:1$ (Normal) | **Pass (AAA)** |
| Dark Muted Text | `#AAB3AD` | `#171C19` | **8.02:1** | $\ge 4.5:1$ (Normal) | **Pass (AAA)** |
| Dark Accent | `#EF8B79` | `#171C19` | **7.10:1** | $\ge 4.5:1$ (Normal) | **Pass (AAA)** |
| Dark On-Accent Text | `#171C19` | `#EF8B79` | **7.10:1** | $\ge 4.5:1$ (Normal) | **Pass (AAA)** |
| Dark Strong Border | `#6E7C74` | `#171C19` | **3.94:1** | $\ge 3.0:1$ (UI / Boundary) | **Pass (AA)** |

---

## 6. Social Card Export Verification

- **Export Path**: `assets/brand/social/card.png`
- **Raster Dimensions**: Exactly `1200 × 630 px`.
- **Color Mode**: `RGB` (8-bit per channel).
- **Visual Agreement**: Renders identical layout, typography, grid rules, registration marks, and badges to `card.svg`.
- **Factual Claims**:
  - Badge: `RESEARCH PREVIEW`
  - Headline: `Native vector search, in development.`
  - Description: `Qenlo is a Rust vector-database research project for native applications.`
  - Zero unsupported benchmark claims, zero competitor references.

---

## 7. Limitations &amp; Disclaimers

1. **Trademark Clearance**: This quality assurance audit certifies technical SVG geometry, font licensing, WCAG color contrast, and offline asset integrity. It does **not** constitute legal trademark clearance or copyright registration.
2. **Platform Controls**: Native desktop and mobile application controls may render using native OS platform fonts (e.g. Segoe UI on Windows, San Francisco on macOS/iOS) rather than IBM Plex Sans.
3. **Research Preview Scope**: All benchmark tables and performance figures in specimen sheets are explicitly placeholders labeled *"Not yet measured"* to reflect the early prototype phase of the project.
