# Design system

Spudbox's colors are a set of semantic CSS custom properties defined in
`src/lib/styles/theme.css`, scoped per theme under `:root[data-theme="..."]`.
Components never use a literal hex/rgba color for anything that represents
app chrome (surfaces, text, borders, accents, status) — they reference a
token, and the active theme decides what that token resolves to. This is
what makes adding a theme a `theme.css`-only change.

## Token categories

| Token | Used for |
|---|---|
| `--bg-base` | The main content background (album grid, track list). |
| `--bg-elevated` | Static raised panels: sidebar, transport bar, popovers, drawers. |
| `--bg-hover` | Resting background for interactive controls (buttons, inputs, list rows) — despite the name, this is the *default* state, not `:hover`. |
| `--bg-selected` | The active/hover/pressed state for those same controls, and for selected list items. |
| `--border` | Hairline borders and dividers. |
| `--text-primary` / `--text-secondary` / `--text-tertiary` | Decreasing emphasis: headings/values, body/labels, hints/disabled. |
| `--accent` / `--accent-hover` | The theme's primary color and its hover/active state. |
| `--accent-contrast` | Text/icon color to place on a solid `--accent` fill (a button label, a filled badge). Never assume white — see below. |
| `--success` / `--danger` / `--warning` | Semantic status colors (connected/added, error/remove, caution). Independent of the accent hue except where noted. |
| `--success-bg` / `--danger-bg` / `--warning-bg` / `--*-border` | Tinted badge/panel backgrounds, *derived* from the base status color — see below, not set per-theme. |

Non-color tokens (`--radius`, `--radius-sm`, `--sidebar-width`,
`--transport-height`, `--toolbar-height`) are structural, not visual
identity, so they live once in the un-themed part of `:root` and are shared
by every theme.

## Media scrims are theme-invariant

The translucent black overlays used to dim album art (hover states, the
lightbox backdrop, the now-playing drawer backdrop) are **not** tokenized
per-theme and stay as literal `rgba(0, 0, 0, …)`. They composite over
photographic album art or dim the whole app behind a modal — a black scrim
reads as "dimmed" regardless of which chrome theme is active, the same way
a photo viewer's lightbox backdrop doesn't change color with OS light/dark
mode. Tokenizing them would add indirection with no real payoff. If a
future scrim needs theme awareness (e.g. a scrim over a *non-photographic,
theme-colored* panel), give it a real token instead of extending this rule.

## Tinting a colored theme's neutrals

The first pass at `green`/`purple`/`yellow` just swapped `--accent` on top
of `dark`'s exact neutral gray scale and left it there — every colored
theme ended up feeling like the same theme wearing a different badge color,
not a distinct one. The fix: each colored theme's `--bg-*`/`--border`/
`--text-*` tokens carry a low, elevation-ramped tint of that theme's own
accent hue, instead of reusing `dark`'s neutral gray.

Concretely, start from `dark`'s neutral scale in HSL, then replace
hue+saturation with the theme's accent hue at a ramp that *increases with
elevation/interactivity* — the same "moves toward the foreground" idea
used for `--accent-hover` — and widen the lightness gaps between tokens a
little rather than reusing `dark`'s exact lightness values. The wider
lightness spread plus real saturation (not just a whisper) is what makes
the theme read as a distinct mood rather than "dark theme, different
accent dot"; check contrast after, don't assume it survives unchanged:

| token | saturation | lightness (vs. `dark`'s) |
|---|---|---|
| `--bg-base` | ~20% | slightly lower (deeper) |
| `--bg-elevated` | ~24% | about the same |
| `--bg-hover` | ~30% | higher |
| `--bg-selected` / `--border` | ~32–36% | higher still |
| `--text-tertiary` | ~18% | about the same |
| `--text-secondary` | ~12% | about the same |
| `--text-primary` | ~5% (kept close to neutral — this is the main reading color, and a strong tint here fights legibility) | about the same |

At this saturation, `text-tertiary`'s contrast against `bg-base` in
particular is worth re-checking every time (it's the token with the least
margin above its 3:1 floor) — it stayed comfortably clear (3.3–4.4:1
measured) across all three colored themes here, but a hue/lightness choice
that pushes it under 3:1 means back off saturation or nudge lightness, not
skip the check.

`light` doesn't get this treatment: it's meant to read as the neutral
inverse of `dark`, not a sixth "colored" theme, so its grays stay genuinely
gray.

## Choosing a new accent color

1. **Pick a hue**, not a stock color name. Muted, not saturated: keep
   saturation roughly in the 25–40% range in HSL. Above that it starts
   reading as a UI toy rather than a calm, long-session music player.
2. **Lightness depends on what the accent sits on.** Spudbox's colored
   themes (green/purple/yellow) sit on a near-black neutral scale (tinted
   per the section above, but still near-black in lightness), so their
   accent lightness should land in the same range `dark`'s accent uses
   (roughly L 50–65%) — light enough to read against that near-black
   surface, dark enough to stay muted. A theme built on a light neutral
   scale (like `light`) needs a *darker* accent (roughly L 40–55%) for the
   same reason in reverse.
3. **Derive `--accent-hover` by moving toward that theme's text-primary,
   not by a fixed rule of "lighter."** In `dark` and the three colored
   themes, hover moves *lighter* (toward white text) — interaction reads as
   "brighter/more foreground." In `light`, hover moves *darker* (toward
   black text) — the conventional light-UI "pressed" feel. Concretely:
   lighten by ~12–15% toward white for dark-neutral themes, darken by
   ~12–15% toward black for `light`.
4. **Compute `--accent-contrast`, don't assume white.** A muted,
   moderate-lightness accent (the whole point of rule 1) frequently fails
   4.5:1 against white text — measured contrast for all three colored
   themes here lands between 2.5:1 and 3.5:1 with white, but 5:1–7.5:1 with
   a near-black ink (`#15151a`). Check both and pick whichever clears
   4.5:1; for muted accents at moderate lightness that will almost always
   be dark ink, not white.
   - **Known exception:** `dark`'s accent (`#818cf8`) predates this rule
     and uses white text at ~3:1 contrast. It's kept as-is because `dark`
     is a direct port of Spudbox's original, already-shipped palette, not
     a place to introduce a visible button-color change as a side effect
     of writing this document. Don't copy the exception into new themes.

## Status colors vs. the accent hue

`--success` / `--danger` / `--warning` carry fixed meaning (added/connected,
error/remove, caution) and are chosen independently of the theme's accent —
*except* when a theme's accent shares the status color's hue family, which
would make an "active" element and a "connected" badge look like the same
color. Concretely in this codebase: the `green` theme's accent is itself a
green, so `--success` there is shifted toward emerald/teal (`#5bc7a0`)
rather than reusing the grass-green used everywhere else, and the `yellow`
theme's accent sits close to the default warning hue, so `--warning` there
is shifted toward orange (`#e08a55`). Apply the same check for any future
themes: if a new accent's hue falls within roughly 30° of a status color's
hue *and* they're used on the same dark-neutral scale, shift the status hue
until they're visually distinct at a glance, not just by looking at the hex
values.

## Deriving tinted status backgrounds/borders

`--success-bg`, `--danger-bg`, `--warning-bg` and their `-border` variants
are not set per-theme directly. They're computed once, in the shared part
of `theme.css`, from the base status color and whatever surface it's
sitting on:

```css
--success-bg: color-mix(in srgb, var(--success) 18%, var(--bg-elevated));
--success-border: color-mix(in srgb, var(--success) 45%, var(--bg-elevated));
```

Mixing against `--bg-elevated` (rather than a fixed white/black) is what
makes this formula work unmodified across both near-black and near-white
themes: on `dark` it produces a dark, faintly-tinted badge background; on
`light` the exact same formula produces a pale, faintly-tinted one — no
per-theme background tuning required. Only the base `--success` /
`--danger` / `--warning` hue needs a value per theme; the tint math is
universal.

## Contrast minimums

Following the project's accessibility requirement, every token pairing
that's actually used together is checked against WCAG 2.1 numbers before
it ships, not eyeballed:

- `--text-primary` on `--bg-base`: **≥ 7:1** (this is the main reading
  pair — comfortable margin, not just AA).
- `--text-secondary` on whatever surface it appears on: **≥ 4.5:1** (AA,
  normal text).
- `--text-tertiary` on whatever surface it appears on: **≥ 3:1** (AA,
  reserved for hints/disabled/decorative text, never body copy).
- Any status color used as text/icon (`.badge`, `.change-icon`, `.message`)
  against the surface it sits on: **≥ 4.5:1**.
- `--accent-contrast` on `--accent`: **≥ 4.5:1**.
- Decorative-only borders (e.g. the destructive button's resting border)
  are held to the UI-component 3:1 bar on a best-effort basis, not treated
  as load-bearing — they're always paired with a passing text/icon color
  that carries the actual meaning.

When adding a theme, compute these before wiring it into the switcher, the
same way this document's candidates were checked (relative-luminance
contrast ratio, not "looks fine").

## Adding a new theme, end to end

1. Pick/derive the 14 per-theme tokens above (surfaces × 4, border, text × 3,
   accent × 3, status base × 3) following the rules above — for a theme
   meant to sit on the near-black scale (like green/purple/yellow), that
   means tinting the neutrals to the new hue, not reusing `dark`'s.
2. Add a `:root[data-theme="yourtheme"] { … }` block to `theme.css`. Don't
   touch the shared derived-formula block.
3. Add the theme to the `Theme` union and `THEMES` list in
   `src/lib/stores/theme.svelte.ts` — the quick-access switcher (the
   palette icon next to the settings cog; there's deliberately no second
   copy of this control in Settings) reads from that single list. The
   switcher's swatch dots are the one deliberate exception to "always use a
   token": they show every theme's accent at once, so each dot is keyed off
   a literal hex matching that theme's `--accent` in `theme.css`, not a
   `var()` (the page can only have one theme's tokens active at a time).
   Keep those two in sync by hand when a theme's accent changes.
4. Load it in the running app and sanity-check every screen — badges,
   destructive buttons, the equalizer curve, star ratings — not just the
   toolbar and sidebar.
