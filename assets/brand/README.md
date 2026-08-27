# Qenlo Brandkit — Fonts & Design System Tokens

This directory contains the foundational design tokens, typography assets, and palette definitions for the Qenlo platform.

---

## 1. Typography & Web Font Assets

All font assets are licensed under the **SIL Open Font License (OFL) v1.1** (see [fonts/OFL.txt](file:///D:/qenloDB/assets/brand/fonts/OFL.txt)).

### Font Files
- **IBM Plex Sans Regular**: [`fonts/IBMPlexSans-Regular.woff2`](file:///D:/qenloDB/assets/brand/fonts/IBMPlexSans-Regular.woff2)
  - Upstream Package: `@ibm/plex-sans@1.1.0`
  - Upstream URL: `https://raw.githubusercontent.com/IBM/plex/master/packages/plex-sans/fonts/complete/woff2/IBMPlexSans-Regular.woff2`
  - Release Tag: `@ibm/plex-sans@1.1.0`
- **IBM Plex Sans SemiBold**: [`fonts/IBMPlexSans-SemiBold.woff2`](file:///D:/qenloDB/assets/brand/fonts/IBMPlexSans-SemiBold.woff2)
  - Upstream Package: `@ibm/plex-sans@1.1.0`
  - Upstream URL: `https://raw.githubusercontent.com/IBM/plex/master/packages/plex-sans/fonts/complete/woff2/IBMPlexSans-SemiBold.woff2`
  - Release Tag: `@ibm/plex-sans@1.1.0`
- **IBM Plex Mono Regular**: [`fonts/IBMPlexMono-Regular.woff2`](file:///D:/qenloDB/assets/brand/fonts/IBMPlexMono-Regular.woff2)
  - Upstream Package: `@ibm/plex-mono@2.5.0`
  - Upstream URL: `https://raw.githubusercontent.com/IBM/plex/master/packages/plex-mono/fonts/complete/woff2/IBMPlexMono-Regular.woff2`
  - Release Tag: `@ibm/plex-mono@2.5.0`

---

## 2. Design System Tokens

Design tokens are maintained with 100% parity across:
1. **JSON Format**: [`tokens.json`](file:///D:/qenloDB/assets/brand/tokens.json) (DTCG standard)
2. **CSS Variables**: [`tokens.css`](file:///D:/qenloDB/assets/brand/tokens.css) (`--qenlo-*` namespace)

### Color Palette

| Token Key | Light Mode | Dark Mode | Role / Semantic Use |
|---|---|---|---|
| `background` | `#F7F5F0` | `#171C19` | Main canvas background |
| `text` | `#1D2320` | `#F7F5F0` | Primary body and heading text |
| `muted-text` | `#5A615D` | `#AAB3AD` | Subtitles, metadata, secondary labels |
| `accent` | `#B53C2F` | `#EF8B79` | Primary brand accent & active state |
| `on-accent` | `#FFFFFF` | `#171C19` | Text/glyph on top of accent fill |
| `surface` | `#EFECE6` | `#1F2622` | Cards, panels, toolbars |
| `surface-raised` | `#E6E2DA` | `#2A332E` | Modals, popovers, dropdown menus |
| `border` | `#D8D3C8` | `#2E3833` | Subtle container and card outlines |
| `border-strong` | `#878E8A` | `#6E7C74` | Active input and high-contrast borders |
| `focus` | `#B53C2F` | `#EF8B79` | Focus ring indicator outline |

### Typography

- **Sans Stack**: `'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`
- **Mono Stack**: `'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace`
- **Font Weights**:
  - `regular`: `400`
  - `semibold`: `600`

### Type Scale

| Scale Key | Rem Value | Pixel Value | Line Height |
|---|---|---|---|
| `xs` | `0.75rem` | `12px` | `1rem` (16px) |
| `sm` | `0.875rem` | `14px` | `1.25rem` (20px) |
| `base` | `1rem` | `16px` | `1.5rem` (24px) |
| `lg` | `1.125rem` | `18px` | `1.75rem` (28px) |
| `xl` | `1.375rem` | `22px` | `1.875rem` (30px) |
| `2xl` | `1.75rem` | `28px` | `2.25rem` (36px) |
| `3xl` | `2.25rem` | `36px` | `2.75rem` (44px) |

### Spacing Scale (4/8-based)

| Space Key | Rem Value | Pixel Value |
|---|---|---|
| `1` | `0.25rem` | `4px` |
| `2` | `0.5rem` | `8px` |
| `3` | `0.75rem` | `12px` |
| `4` | `1rem` | `16px` |
| `6` | `1.5rem` | `24px` |
| `8` | `2rem` | `32px` |
| `12` | `3rem` | `48px` |
| `16` | `4rem` | `64px` |

---

## 3. Validation & Parity Check

To run the automated verification script:
```bash
python assets/brand/verify_tokens_and_fonts.py
```
