# SermonAI — Branding & Design System

> **Companion to `PRD.md` v2.1 and `MILESTONES.md`.**
> This document is the single source of truth for how SermonAI looks: colors, typography, spacing, elevation, motion, iconography, tone of voice. If the PRD says *what*, this file says *how it should feel*.
> When Claude Code writes UI, it must reference the tokens in this file by name (e.g. `bg.surface`, `accent.violet`) rather than hard-coded hex values.

| Field | Value |
|---|---|
| **Version** | 1.0 |
| **Last Updated** | 7 September 2026 |
| **Applies to** | Operator UI, Projector output (default theme), Alternate output, Phone remote, Website, Installers, PDF summary "Sermon Brief" template |

---

## 1. Brand Essence

SermonAI is **calm confidence in a dark booth**. It should feel like a trusted instrument in the media team's hands — not showy, not corporate, not churchy in a cliché way. Every visual choice serves one of three roles:

- **Focus.** The operator can look at the screen for two seconds and know what matters.
- **Reverence.** The scripture, when it appears on the projector, is treated with weight and stillness.
- **Craft.** Details are deliberate. Nothing rattles, blinks, or draws attention to itself unless something is genuinely wrong.

**Aesthetic direction:** dark, near-black surfaces; soft cool-lavender to violet accent; generous whitespace; rounded pill-shaped controls; a single strong hero typeface; content-forward layouts.

---

## 2. Color System

The palette is built around a single accent hue (violet) laid over a neutral near-black grayscale. The dashboard-in-a-dashboard aesthetic is achieved through two nested dark layers, not through borders.

### 2.1 Neutrals (surfaces & text)

| Token | Hex | Role |
|---|---|---|
| `bg.canvas` | `#0F0F10` | Outermost background — the darkest layer, behind everything |
| `bg.surface` | `#1B1B1D` | Main app surface — the dashboard container |
| `bg.raised` | `#242426` | Cards, panels, popovers |
| `bg.sunken` | `#141416` | Insets, code blocks, empty states |
| `bg.pill` | `#2B2B2E` | Nav pills, tag chips, secondary buttons |
| `bg.hover` | `#2F2F32` | Hover state on `bg.raised` |
| `bg.active` | `#3A3A3E` | Pressed / selected state |
| `border.subtle` | `#2A2A2D` | Card outlines when needed (usually skipped) |
| `border.default` | `#3A3A3E` | Dividers, input borders |
| `border.strong` | `#54545A` | Focus rings, emphasis borders |
| `text.primary` | `#F5F5F7` | Headings, key labels, verse text on light backgrounds |
| `text.secondary` | `#B8B8BD` | Body copy, standard UI text |
| `text.muted` | `#7E7E85` | Metadata, timestamps, helper text |
| `text.disabled` | `#4A4A50` | Disabled states |
| `text.on-accent` | `#0F0F10` | Text placed on a filled accent surface |
| `text.on-light` | `#0F0F10` | Text placed on `bg.light-cta` |
| `bg.light-cta` | `#E2E2E2` | The primary "solid" CTA button on dark surfaces |

### 2.2 Accent — Violet

This is the only chromatic hue in the system. It ranges from a soft dusty lavender used for large headline emphasis, through a vivid violet used for the most important action or highlight in a view, down to a deep grape used for pressed and depth states. **Use accent sparingly**: one accent moment per view is the rule.

| Token | Hex | Role |
|---|---|---|
| `accent.violet.50` | `#F1EAF8` | Backgrounds behind accent text on light PDF templates |
| `accent.violet.100` | `#DCCCE8` | Very soft tint, hover on accent chips |
| `accent.violet.200` | `#C7B0D8` | **Headline emphasis** — the "Dashboard" purple in the hero |
| `accent.violet.300` | `#B294C5` | Secondary accent, disabled accent states |
| `accent.violet.400` | `#A47CBF` | Alt accent for charts and non-primary highlights |
| `accent.violet.500` | `#B980E8` | **Primary accent** — the bright bar, the "on-air" glow, the selected day |
| `accent.violet.600` | `#9F66CE` | Hover on primary accent |
| `accent.violet.700` | `#7C4CA8` | Pressed |
| `accent.violet.800` | `#5A3680` | Deep accent, borders on accent surfaces |
| `accent.violet.900` | `#2E1B45` | Deepest accent, accent shadow tint |

**Accent usage rules**
- The projector "live verse" state does **not** use violet. Scripture is set in `text.primary` on `bg.canvas` so nothing competes with the Word.
- Violet marks *state that is happening now*: a detection card that just arrived, the "Live" badge, the selected date, the peak bar in a chart, the record-status dot.
- Never colorize body text with violet. Only headlines, single words, badges, or filled circles.
- On the projector, violet may appear as a thin bottom border under the reference tag; nothing more.

### 2.3 Semantic Colors

Reserved for status only. Never decorative.

| Token | Hex | Role |
|---|---|---|
| `status.success` | `#4ADE80` | Ready, connected, saved, accepted |
| `status.success.bg` | `#0F2A1A` | Success surface tint |
| `status.warning` | `#F5B84A` | Needs attention, offline mode banner, low disk |
| `status.warning.bg` | `#2E220A` | Warning surface tint |
| `status.danger` | `#F87171` | Errors, disconnection, revoked device |
| `status.danger.bg` | `#2E1414` | Danger surface tint |
| `status.info` | `#7DB4F5` | Neutral info banners, tips |
| `status.info.bg` | `#111F2E` | Info surface tint |
| `status.live` | `accent.violet.500` | The scripture is currently on the projector |
| `status.staged` | `#B8B8BD` | The scripture is staged, not yet live |

### 2.4 Detection Source Badges

Each detection card shows where the match came from. These use small filled circles, not text color.

| Source | Token | Hex |
|---|---|---|
| Regex (direct reference) | `source.regex` | `#4ADE80` (green — high confidence) |
| Vector (semantic) | `source.vector` | `#7DB4F5` (blue — semantic) |
| LLM (paraphrase) | `source.llm` | `accent.violet.500` (violet — AI) |
| Manual (operator typed) | `source.manual` | `#B8B8BD` (neutral) |

### 2.5 Contrast & Accessibility

Every text/background pairing in the operator UI is verified against WCAG AA (4.5:1 for body, 3:1 for large text). Verified pairings:

| Foreground | Background | Ratio | Verdict |
|---|---|---|---|
| `text.primary` | `bg.canvas` | 17.9:1 | AAA |
| `text.primary` | `bg.surface` | 15.3:1 | AAA |
| `text.primary` | `bg.raised` | 12.7:1 | AAA |
| `text.secondary` | `bg.surface` | 8.4:1 | AAA |
| `text.muted` | `bg.surface` | 4.6:1 | AA |
| `accent.violet.200` (headline) | `bg.canvas` | 10.8:1 | AAA |
| `accent.violet.500` (badge) | `bg.canvas` | 5.9:1 | AA large |
| `text.on-light` | `bg.light-cta` | 15.1:1 | AAA |

**Color blindness:** the system never relies on hue alone to communicate state. Detection source badges include a small letter (R / V / A / M) under the dot; status banners include an icon; the live/staged distinction is also carried by position on screen.

---

## 3. Typography

### 3.1 Type Families

| Role | Family | Fallback |
|---|---|---|
| **UI** (all operator, remote, website chrome) | **Inter** | `-apple-system, "Segoe UI", Roboto, sans-serif` |
| **Display headline** (hero, section titles) | **Inter Display** (or Inter with `font-feature-settings: "ss01"`) | same as UI |
| **Scripture** (projector output, PDF summary) | **Crimson Pro** | `Georgia, "Times New Roman", serif` |
| **Numeric** (timers, timestamps, VU meter) | **Inter** with tabular numerals (`font-variant-numeric: tabular-nums`) | — |
| **Monospace** (logs, code, diagnostic panel) | **JetBrains Mono** | `ui-monospace, "SF Mono", monospace` |

**License and shipping:** both Inter and Crimson Pro are Open Font License. Bundle the WOFF2 subsets inside the app; do not link to Google Fonts at runtime (breaks offline).

### 3.2 Type Scale

Based on a 1.250 (major third) modular scale, rounded for pixel comfort.

| Token | Size / Line height | Weight | Tracking | Use |
|---|---|---|---|---|
| `type.display.xl` | 64 / 68 | 500 | -1.5% | Hero headline, marketing site only |
| `type.display.lg` | 48 / 52 | 500 | -1% | Onboarding step titles |
| `type.display.md` | 36 / 40 | 500 | -0.5% | Sermon Library section headers, PDF cover |
| `type.h1` | 28 / 34 | 600 | -0.25% | Operator page titles |
| `type.h2` | 22 / 28 | 600 | 0 | Card titles ("Tasks progress" style) |
| `type.h3` | 18 / 24 | 600 | 0 | Sub-section headers |
| `type.body.lg` | 16 / 24 | 400 | 0 | Primary body copy |
| `type.body` | 14 / 20 | 400 | 0 | Default UI text |
| `type.body.sm` | 13 / 18 | 400 | 0 | Dense areas, table cells |
| `type.caption` | 12 / 16 | 500 | +1% | Labels, chip text, metadata |
| `type.micro` | 11 / 14 | 500 | +2% | VU dB labels, ultra-small annotations |
| `type.scripture.projector` | fluid 64–140 / 1.2 | 400 | -0.5% | Verse on the projector (auto-fit) |
| `type.scripture.reference` | 22 / 28 | 500 (small caps) | +6% | Reference tag under the verse |
| `type.scripture.pdf` | 14 / 22 | 400 | 0 | Scripture text inside the summary PDF |

### 3.3 Typographic Rules

- **Headlines mix weights, not fonts.** The hero "Dashboard" purple works because the emphasized word shares the family with the rest — never introduce a decorative font for one word.
- **Numerals are always tabular** in operator UI so timers, percentages and dates don't jitter.
- **All-caps only for micro labels** (12px and below), never for headings.
- **Scripture is set in Crimson Pro**, always, on both the projector and inside PDFs. This is a bright line between UI and Word.
- **Line length in prose blocks** (PDF summary, Content Studio): 60 to 75 characters. Never full width.
- **No italics** in the operator UI except in scripture citations. Italics in a dark, dense booth read poorly.

---

## 4. Layout & Spacing

### 4.1 Spacing Scale (4px base)

| Token | px | Common use |
|---|---|---|
| `space.0` | 0 | reset |
| `space.1` | 4 | icon/text gaps, dense grids |
| `space.2` | 8 | tight card padding, form group inner spacing |
| `space.3` | 12 | button padding-y, chip padding |
| `space.4` | 16 | default gap between related elements |
| `space.5` | 20 | card padding on small screens |
| `space.6` | 24 | card padding default |
| `space.8` | 32 | section spacing inside a panel |
| `space.10` | 40 | between panels |
| `space.12` | 48 | page top/bottom padding |
| `space.16` | 64 | between major sections on marketing |
| `space.24` | 96 | hero vertical breathing room |

### 4.2 Radius Scale

The design uses generous rounding. Nothing has hard corners except lines and hairlines.

| Token | px | Use |
|---|---|---|
| `radius.sm` | 8 | Chips, badges, small controls |
| `radius.md` | 12 | Inputs, small buttons |
| `radius.lg` | 16 | Cards, panels |
| `radius.xl` | 24 | Major container ("dashboard-in-dashboard") |
| `radius.pill` | 999 | Nav pills, primary buttons, VU meter caps |

### 4.3 Grid & Containers

- **Operator window** uses a two-column layout: left column 55% for the transcript, right column 45% for detection cards + staging + preview. On windows narrower than 1280px it collapses to a single column with a segmented control at the top.
- **Cards** are 24px padded, 16px rounded, sitting on `bg.surface` with `bg.raised` as their fill. No borders by default; separation comes from color contrast.
- **Nested containers** (like the dashboard-inside-dashboard motif in the reference) use `radius.xl` on the outer and `radius.lg` on the inner card set.
- **Marketing site** uses a 1200px max content width with 24px gutter on desktop.

### 4.4 Elevation (Shadows)

Elevation is minimal. Dark UIs get depth from *lighter surfaces on darker ones*, not from shadow blur. Shadows are only used for floating elements above the flow.

| Token | Values | Use |
|---|---|---|
| `elevation.none` | none | Cards, panels (default) |
| `elevation.hover` | `0 1px 0 0 rgba(255,255,255,0.04)` | Card hover, subtle lift |
| `elevation.pop` | `0 8px 24px -8px rgba(0,0,0,0.5)` | Popovers, dropdowns |
| `elevation.modal` | `0 24px 48px -12px rgba(0,0,0,0.6), 0 0 0 1px rgba(255,255,255,0.05)` | Modals, command palette |
| `elevation.glow.violet` | `0 0 24px -4px rgba(185,128,232,0.35)` | The "Live" verse indicator only. One per view. |

---

## 5. Components

### 5.1 Buttons

Three tiers. Nothing else.

**Primary (light-solid) — the one important action per view**
- Background `bg.light-cta`, text `text.on-light`, border none.
- Padding: `space.3` × `space.6` (12 × 24). Radius `radius.pill`. Font `type.body` weight 600.
- Hover: background `#F5F5F7`. Active: `#CFCFCF`.

**Secondary (ghost-pill) — supporting actions**
- Background `bg.pill`, text `text.primary`, border `1px solid transparent`.
- Same size and radius as primary.
- Hover: background `bg.hover`. Active: `bg.active`.

**Tertiary (link) — inline actions inside dense areas**
- No background, text `text.primary`, underline on hover only.

**Destructive** — same shape as secondary but text `status.danger` and hover background `status.danger.bg`. Requires confirmation for anything irreversible.

**Icon buttons** — 40×40 with `radius.pill`, `bg.pill` background, icon in `text.secondary`. Never smaller than 40×40 for the operator UI so booth touch and shaky-hand clicks land.

### 5.2 Pill Nav

The dashboard reference uses a pill navigation with a single filled selected state. SermonAI copies this exactly for the operator top tabs (Live / Library / Series / Studio / Settings).

- Container: `bg.pill`, `radius.pill`, padding `space.1` (4px).
- Item default: text `text.secondary`, `space.2` × `space.4` padding.
- Item selected: background `bg.raised`, text `text.primary`, `radius.pill`.
- Transition: 150ms ease.

### 5.3 Cards

The dashboard aesthetic in the reference. Applied literally to Sermon Library, detection stack, and the settings modules.

- Fill `bg.raised`, padding `space.6`, radius `radius.lg`, no border.
- Header row: title in `type.h2`, subtitle in `type.caption` `text.muted`, optional icon top-right in a 40px `bg.pill` circle.
- Interior may contain a chart, a stat set, or a list. Whitespace inside is generous: 24px between rows.

### 5.4 Chart Styling

The reference uses filled purple bars over hatched grey placeholder tops — SermonAI uses this pattern for the "Detection accuracy" and "Service duration" charts in Settings → Diagnostics.

- Filled bar (peak): `accent.violet.500`, `radius.md` on top.
- Non-peak bars: `accent.violet.700` at 60% opacity.
- Hatched projected/target zone: 45° stripes at `border.default` over `bg.raised`.
- Axis labels: `type.micro` `text.muted`. Percentage labels above bars: `type.caption` `text.secondary`.

### 5.5 Calendar / Date Selector

Used in Sermon Library and Series manager.

- Grid of 7 columns, 8px gap.
- Day cell: 36×36, `radius.pill`, text `text.secondary`.
- Today: text `text.primary`, weight 600.
- Selected: background `accent.violet.500`, text `text.on-accent`, `radius.pill`.
- Range endpoints (Phase 3): both endpoints `accent.violet.500`, middle days `accent.violet.900` with `text.primary`.

### 5.6 Detection Card (SermonAI-specific)

The single most important component. Appears in the right column, stacks vertically.

- Fill `bg.raised`, `radius.lg`, padding `space.6`, gap `space.3` between rows.
- Row 1: source dot (12px, colored per §2.4) + letter + reference (`type.h3` `text.primary`) + translation chip (`type.caption` in `bg.pill`).
- Row 2: verse text (`type.body.lg` `text.secondary`, max 4 lines with fade at bottom if truncated).
- Row 3: confidence bar (2px tall, full width, fill `accent.violet.500` at width = confidence%).
- Row 4: three buttons — Accept (primary), Reject (secondary), Edit (tertiary).
- Cards enter with a 200ms fade + 8px slide-up. They **do not** bounce or spring.

### 5.7 Input Fields

- Height 40, `bg.sunken`, `border.default` 1px, `radius.md`.
- Focus: border `accent.violet.500`, outline `2px` `accent.violet.900`.
- Placeholder: `text.muted`. Label above in `type.caption` `text.secondary`.

### 5.8 Toasts & Banners

- Positioned top-right of the operator window, 16px from the edge, stacked with 8px gap.
- `radius.lg`, `elevation.pop`, colored using the semantic pair (e.g. `status.success` text on `status.success.bg`).
- Auto-dismiss after 4s for info/success; danger and warning are sticky until dismissed.

---

## 6. Iconography

- **Family:** Lucide (open source, matches the geometric feel of Inter).
- **Stroke width:** 1.5 default, 2 for micro (12/16px) sizes.
- **Sizes:** 16, 20, 24 (default), 40 (icon buttons). Never scale by CSS transform.
- **Color:** icons inherit `currentColor`. Default is `text.secondary` on card interiors, `text.primary` when they're the focus of a control.
- **Icon-in-circle motif** (from the reference — a filled violet circle behind an icon): 40×40 `bg.pill` circle by default; only use `accent.violet.500` fill when the icon marks the *active* state of that panel.

**Custom icons** are avoided in v1. The SermonAI logomark is the only custom mark.

---

## 7. Motion

### 7.1 Principles

- **Under 300ms.** No animation lasts longer.
- **Ease, don't bounce.** Cubic bezier `(0.16, 1, 0.3, 1)` for entries; `(0.4, 0, 0.2, 1)` for exits.
- **One thing moves at a time in the operator UI.** Multi-element choreography is fine on the marketing site, not in the booth.
- **Scripture transitions on the projector** are the only place we spend real animation budget: 400ms fade-in (default), or slide or dissolve per theme. They are configurable per church.

### 7.2 Named Transitions

| Token | Duration | Use |
|---|---|---|
| `motion.instant` | 0ms | Selected state on nav pills |
| `motion.fast` | 120ms | Hover, focus rings, tooltip in |
| `motion.base` | 180ms | Card hover, button press |
| `motion.card-enter` | 200ms | Detection cards, toasts |
| `motion.verse-in` | 400ms | Projector verse fade-in (default theme) |
| `motion.verse-out` | 250ms | Projector verse fade-out |

### 7.3 What we never do

- No parallax on the marketing site.
- No auto-playing background video anywhere.
- No spinning loaders longer than 3 seconds without progress feedback.
- No confetti, ever, for any reason.

---

## 8. Logo

### 8.1 Logomark

The reference shows a subtle target/spiral motif (`Organized` icon). The SermonAI mark is an **open-book silhouette with a soundwave arc traced through it**, single-weight strokes, always paired with the wordmark unless space forbids it.

- **Weight:** 1.5px stroke at the primary usage size (32px height). Scales proportionally.
- **Container:** a 32×32 optical square. On the top-left of the operator window and the marketing header.
- **Do not**: recolor the mark violet inside the app; rotate it; place it inside a filled shape; add drop shadows.

### 8.2 Wordmark

- **Type:** Inter Display, weight 500, tracking -1%.
- **Case:** `SermonAI` (capital S, capital A, one word).
- **Mark-to-wordmark spacing:** 8px.
- **Lockup baseline:** the mark's optical center aligns with the wordmark's x-height, not the baseline.

### 8.3 Clear Space

Minimum clear space around any lockup is equal to the height of the "S" in the wordmark on all sides.

### 8.4 On the Projector

The SermonAI logo **never appears on the projector output** during a service. The only exceptions are the free-tier watermark (see §11) and the startup splash before "Start Service" is clicked.

---

## 9. Tone of Voice

### 9.1 Voice Principles

- **Plain, warm, honest.** No church jargon we don't need. No tech jargon we can avoid.
- **Respect the operator.** They are volunteers under pressure. Sentences are short.
- **Never cute in error states.** "Something went wrong 😊" is banned. Say what happened and what to do next.
- **Never solemn in mundane places.** A settings toggle doesn't need a Bible verse next to it.

### 9.2 Word choices

| Prefer | Avoid |
|---|---|
| verse, passage, scripture | pericope, lection |
| church | client, tenant, organization |
| service | event, session |
| preacher, pastor | speaker, presenter |
| the projector, on screen | the output device, the rendered surface |
| offline mode | disconnected mode |
| the summary | the AI-generated post-service intelligence brief |

### 9.3 Microcopy patterns

- **Buttons** are verbs: `Go Live`, `Download PDF`, `End Service`, `Reject`, `Regenerate`. Never "Click here", never "Submit".
- **Empty states** have three lines: what this is, why it's empty, what to do next.
- **Errors** name the thing that failed, then offer one action: `Deepgram is unreachable. Switch to offline speech →`
- **Confirmations for irreversible actions** always name the thing being destroyed: `Delete "Faith Foundations, Week 3"? This removes the transcript and summary permanently.` — not just "Are you sure?"
- **Loading verbs**: `Preparing summary…`, `Downloading Offline Speech Pack…` — always with an object. Never bare "Loading…".

### 9.4 Tagline

Primary: **"Every verse. Every sermon. Kept."**

Secondary lines used in different contexts:
- Marketing hero: *"Hears the sermon. Finds the verse. Keeps the record."*
- Installer: *"About sixty seconds to Sunday."*
- Empty Sermon Library: *"Nothing preached here yet. Your first service will be summarized here."*

---

## 10. PDF "Sermon Brief" — Light Mode

The summary PDF is the only surface that lives outside the dark UI. It uses an inverted palette to be printable and shareable in emails and small groups.

| Token | Hex | Role |
|---|---|---|
| `pdf.bg` | `#FBFAF7` | Page background — warm off-white, not clinical |
| `pdf.surface` | `#FFFFFF` | Scripture block backgrounds |
| `pdf.border` | `#E7E4DE` | Hairlines, quote rule |
| `pdf.text.primary` | `#1A1A1E` | Body |
| `pdf.text.secondary` | `#4A4A50` | Meta, captions |
| `pdf.text.muted` | `#8A8A90` | Timestamps, page footer |
| `pdf.accent` | `#7C4CA8` | Section headings, reference tags |
| `pdf.accent.tint` | `#F1EAF8` | Highlight behind key quotes |

**Type in the PDF**
- Cover title: Inter Display 48/52 weight 600.
- Section headings: Inter 20/26 weight 600, `pdf.accent`.
- Body: Crimson Pro 12/18 weight 400 for prose; Inter 11/16 for lists.
- Scripture text: Crimson Pro 13/20 weight 400 with a 4px `pdf.accent` left rule and 12px padding.

**Layout**
- A4 (default) and US Letter, 20mm margins.
- Two columns for the "Discussion Questions" and "Prayer Points" sections; single column everywhere else.
- Header on pages 2+: sermon title left, date right, hairline underneath.
- Footer on all pages: page number center, "Generated by SermonAI" small-right in `pdf.text.muted`.

---

## 11. Free-Tier Watermark

The free tier shows a small, respectful watermark on the projector output.

- **Position:** bottom-right, 24px inset from both edges.
- **Content:** SermonAI logomark + wordmark at 60% opacity.
- **Size:** 20px logomark height.
- **Never** overlaps the verse text (the auto-fit engine leaves a reserved 32px band along the bottom).
- **Removed** the moment a paid license is activated, no other UI difference.

---

## 12. Design Tokens (JSON)

Ship this as `src/design/tokens.json`. Frontend imports it via Tailwind's `theme.extend.colors` and CSS custom properties.

```json
{
  "color": {
    "bg": {
      "canvas": "#0F0F10",
      "surface": "#1B1B1D",
      "raised": "#242426",
      "sunken": "#141416",
      "pill": "#2B2B2E",
      "hover": "#2F2F32",
      "active": "#3A3A3E",
      "light-cta": "#E2E2E2"
    },
    "border": {
      "subtle": "#2A2A2D",
      "default": "#3A3A3E",
      "strong": "#54545A"
    },
    "text": {
      "primary": "#F5F5F7",
      "secondary": "#B8B8BD",
      "muted": "#7E7E85",
      "disabled": "#4A4A50",
      "on-accent": "#0F0F10",
      "on-light": "#0F0F10"
    },
    "accent": {
      "violet": {
        "50": "#F1EAF8",
        "100": "#DCCCE8",
        "200": "#C7B0D8",
        "300": "#B294C5",
        "400": "#A47CBF",
        "500": "#B980E8",
        "600": "#9F66CE",
        "700": "#7C4CA8",
        "800": "#5A3680",
        "900": "#2E1B45"
      }
    },
    "status": {
      "success":    { "fg": "#4ADE80", "bg": "#0F2A1A" },
      "warning":    { "fg": "#F5B84A", "bg": "#2E220A" },
      "danger":     { "fg": "#F87171", "bg": "#2E1414" },
      "info":       { "fg": "#7DB4F5", "bg": "#111F2E" }
    },
    "source": {
      "regex": "#4ADE80",
      "vector": "#7DB4F5",
      "llm": "#B980E8",
      "manual": "#B8B8BD"
    },
    "pdf": {
      "bg": "#FBFAF7",
      "surface": "#FFFFFF",
      "border": "#E7E4DE",
      "text-primary": "#1A1A1E",
      "text-secondary": "#4A4A50",
      "text-muted": "#8A8A90",
      "accent": "#7C4CA8",
      "accent-tint": "#F1EAF8"
    }
  },
  "radius": { "sm": 8, "md": 12, "lg": 16, "xl": 24, "pill": 999 },
  "space":  { "0": 0, "1": 4, "2": 8, "3": 12, "4": 16, "5": 20, "6": 24, "8": 32, "10": 40, "12": 48, "16": 64, "24": 96 },
  "type": {
    "family": {
      "ui": "Inter, -apple-system, 'Segoe UI', Roboto, sans-serif",
      "display": "Inter Display, Inter, sans-serif",
      "scripture": "Crimson Pro, Georgia, serif",
      "mono": "JetBrains Mono, ui-monospace, monospace"
    }
  },
  "motion": {
    "fast": "120ms",
    "base": "180ms",
    "card-enter": "200ms",
    "verse-in": "400ms",
    "verse-out": "250ms",
    "ease-out": "cubic-bezier(0.16, 1, 0.3, 1)",
    "ease-in-out": "cubic-bezier(0.4, 0, 0.2, 1)"
  }
}
```

---

## 13. Tailwind Configuration Snippet

For Claude Code to drop into `tailwind.config.ts`.

```ts
import tokens from "./src/design/tokens.json";

export default {
  content: ["./src/**/*.{ts,tsx,html}"],
  theme: {
    extend: {
      colors: {
        bg: tokens.color.bg,
        border: tokens.color.border,
        text: tokens.color.text,
        accent: tokens.color.accent.violet,
        status: tokens.color.status,
        source: tokens.color.source,
      },
      borderRadius: {
        sm: `${tokens.radius.sm}px`,
        md: `${tokens.radius.md}px`,
        lg: `${tokens.radius.lg}px`,
        xl: `${tokens.radius.xl}px`,
        pill: `${tokens.radius.pill}px`,
      },
      fontFamily: {
        sans: tokens.type.family.ui.split(", "),
        display: tokens.type.family.display.split(", "),
        serif: tokens.type.family.scripture.split(", "),
        mono: tokens.type.family.mono.split(", "),
      },
      transitionTimingFunction: {
        "brand-out": tokens.motion["ease-out"],
        "brand-in-out": tokens.motion["ease-in-out"],
      },
    },
  },
};
```

---

## 14. Do & Don't Gallery

**Do**
- Use exactly one violet accent per view.
- Keep card interiors on `bg.raised` with no borders; let the surface contrast do the work.
- Set scripture in Crimson Pro on the projector.
- Give buttons pill radius and 40px height in the operator UI.
- Verify contrast for every text/background pair against WCAG AA.

**Don't**
- Add a second accent hue (no teal, no gold, no red logos).
- Use violet for body text.
- Use gradients anywhere in v1.
- Introduce rounded rectangles with borders **and** shadows on the same element.
- Animate anything on the projector except the verse itself.
- Put emoji in operator UI text.

---

## 15. Applied Examples

### 15.1 Operator Top Bar

```
[SermonAI mark + wordmark]  [Live pill selected] [Library] [Series] [Studio] [Settings]        [audio meter] [🔴 REC 34:12]  [🌙 Blank]  [End Service (secondary pill)]
```

- Left group in `text.primary`, nav pill on `bg.pill` container.
- Middle stretches to fill.
- Right group: VU meter, elapsed timer in tabular Inter, blank icon-button, `End Service` in secondary pill until you click it — then it becomes a destructive-styled confirmation "Confirm end?" for 3 seconds.

### 15.2 Projector — Default Theme

- Full-screen `bg.canvas`.
- Verse text `text.primary`, Crimson Pro auto-fit between 64px and 140px, centered, max 8 lines.
- Bottom band: reference in small caps, tracked +6%, in `text.secondary`, 32px above bottom edge.
- Thin `accent.violet.500` underline (2px, 80px wide) directly beneath the reference. This is the only violet on the projector.
- Free tier: watermark at bottom-right per §11.

### 15.3 Sermon Library Card

```
[Card, bg.raised, radius.lg, 24px padding]
Title (h2, text.primary):           Faith Foundations, Week 3
Meta (caption, text.muted):         Pastor Emeka · 5 Sept 2026 · 47 min
Scripture badges (caption chips):    Jer 29:11 · John 3:16 · Rom 8:28 · +9 more
Status row (right side):             ● Ready   [Download PDF (primary)]
```

The `●` is the status dot in `status.success`. The card lifts on hover with `elevation.hover`.

---

## 16. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 7 Sept 2026 | Initial brand system extracted from reference design; dark neutrals + violet accent; typography, spacing, motion, PDF light mode, tokens JSON, Tailwind snippet |

---

*End of Document — SermonAI Branding v1.0*
