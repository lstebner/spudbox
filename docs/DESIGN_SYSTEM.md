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
| `--bg-elevated` | Static raised panels: popovers, settings, device sync, the now-playing drawer. (Sidebar and transport bar use `--chrome-*` instead — see "Chrome tokens" below.) |
| `--bg-hover` | Resting background for interactive controls (buttons, inputs, list rows) — despite the name, this is the *default* state, not `:hover`. |
| `--bg-selected` | The active/hover/pressed state for those same controls, and for selected list items. |
| `--border` | Hairline borders and dividers. |
| `--text-primary` / `--text-secondary` / `--text-tertiary` | Decreasing emphasis: headings/values, body/labels, hints/disabled. |
| `--accent` / `--accent-hover` | The theme's primary color and its hover/active state. |
| `--accent-contrast` | Text/icon color to place on a solid `--accent` fill (a button label, a filled badge). Never assume white — see below. |
| `--success` / `--danger` / `--warning` | Semantic status colors (connected/added, error/remove, caution). Independent of the accent hue except where noted. |
| `--success-bg` / `--danger-bg` / `--warning-bg` / `--*-border` | Tinted badge/panel backgrounds, *derived* from the base status color — see below, not set per-theme. |
| `--chrome-bg` / `--chrome-hover-bg` / `--chrome-selected-bg` / `--chrome-border` / `--chrome-text-primary` / `--chrome-text-secondary` / `--chrome-text-tertiary` | The sidebar and transport bar's own background/text colors — see "Chrome tokens" below. |

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

The first pass at `mint`/`grape`/`lemon` (then named `green`/`purple`/
`yellow`) just swapped `--accent` on top of `dark`'s exact neutral gray
scale and left it there — every colored theme ended up feeling like the
same theme wearing a different badge color, not a distinct one. The fix:
each colored theme's `--bg-*`/`--border`/`--text-*` tokens carry a real,
elevation-ramped tint of that theme's own accent hue, instead of reusing a
neutral gray scale. This applies whether the theme sits on a dark neutral
base or a light one — `dark` and `light` themselves are the only two that
stay genuinely gray, since they're meant to read as the neutral pair, not
a "colored" theme.

A second pass then pushed `mint`/`grape`/`lemon` considerably further:
Spudbox is meant to feel "fun and poppy," not restrained or businesslike,
so these three intentionally run at much higher saturation than a typical
"tasteful pastel" palette — see "Choosing a new accent color" below, which
now explicitly rejects the muted-accent rule this section originally
pointed to. `dark` and `light` are unaffected; they're still the calm,
neutral pair.

**Dark-based colored themes** (none currently shipped, but the pattern
still applies if one is added): start from `dark`'s neutral scale in HSL,
then replace hue+saturation with the theme's accent hue at a ramp that
*increases with elevation/interactivity* — the same "moves toward the
foreground" idea used for `--accent-hover` — and widen the lightness gaps
between tokens a little rather than reusing `dark`'s exact lightness
values:

| token | saturation | lightness (vs. `dark`'s) |
|---|---|---|
| `--bg-base` | ~20% | slightly lower (deeper) |
| `--bg-elevated` | ~24% | about the same |
| `--bg-hover` | ~30% | higher |
| `--bg-selected` / `--border` | ~32–36% | higher still |
| `--text-tertiary` | ~18% | about the same |
| `--text-secondary` | ~12% | about the same |
| `--text-primary` | ~5% (kept close to neutral — this is the main reading color, and a strong tint here fights legibility) | about the same |

**Light-based colored themes** (`mint`, `grape`, `lemon`): same idea,
opposite lightness direction — saturation still rises with elevation/
interactivity, but lightness *drops* as you move from `bg-elevated` toward
`bg-hover`/`bg-selected`/`border`, the same "pressed" direction `light`
already uses for its own hover/selected states (see "Choosing a new accent
color" rule 3). `bg-base` itself is deliberately the boldest, most
saturated token — a real, obviously-colored surface, not a whisper — while
`bg-elevated` stays close to white so chrome (sidebar/transport/popovers)
reads as clean structure sitting above it. These three run considerably
hotter than a typical "tasteful pastel" palette on purpose:

| token | saturation | lightness |
|---|---|---|
| `--bg-base` | ~55–75% (bold and vivid — this is the token that should make the theme instantly recognizable, not just tinted) | ~83–89% |
| `--bg-elevated` | ~30–35% | ~98% (near white) |
| `--bg-hover` | ~42–55% | ~90–93% (below `bg-elevated`) |
| `--bg-selected` / `--border` | ~48–65% | ~76–86% (lower still) |
| `--text-tertiary` | ~20–26% | ~40–46% |
| `--text-secondary` | ~18–24% | ~34–36% |
| `--text-primary` | ~18–22% (still kept low relative to the rest — this is the main reading color, and a strong tint here fights legibility even in a poppy palette) | ~11–12% |

Anchoring a token directly on a specific named color (e.g. `lemon`'s
`bg-base` is `#ffff81` exactly, `grape`'s `--accent` is `#6f2da8` exactly)
is a legitimate, even preferred, starting point for this family of themes
— pick the one "this is the color" token that has to be exact, then derive
the rest of the ramp's saturation/lightness around it and verify contrast,
rather than starting from an abstract hue number.

At this saturation, `text-tertiary`'s contrast against `bg-base` in
particular is worth re-checking every time (it's the token with the least
margin above its 3:1 floor) — it measured 3.0–4.0:1 across `mint`/`grape`/
`lemon` on this pass, and 3.3–4.4:1 across the earlier dark-based themes,
but a hue/lightness choice that pushes it under 3:1 means back off
saturation or nudge lightness, not skip the check.

## Choosing a new accent color

1. **Pick a hue**, not a stock color name — ideally anchored on an actual
   named/reference color if one fits the theme's identity (`lemon`'s accent
   direction and `grape`'s accent hex both started this way). For `mint`/
   `grape`/`lemon`, go vivid: saturation in the 75–95% range in HSL, not
   the muted 25–40% range an earlier version of this rule called for.
   Spudbox is meant to read as fun and poppy, not restrained or
   businesslike — a "calm, long-session music player" is `dark`/`light`'s
   job, not every theme's. `dark` and `light` keep their own existing,
   more restrained accents; this rule is specifically for the
   fun-and-poppy family.
2. **Lightness depends on what the accent sits on.** A colored theme built
   on a near-black neutral scale should land its accent lightness in the
   same range `dark`'s accent uses (roughly L 50–65%) — light enough to
   read against that near-black surface, dark enough to stay legible. A
   theme built on a light, boldly-saturated `bg-base` (the `mint`/`grape`/
   `lemon` family) needs a much *darker*, richer accent — roughly L 24–36%
   — so it keeps clearing 4.5:1 against both `bg-base` and the near-white
   `bg-elevated`. Counterintuitively, high saturation plus low lightness is
   what makes an accent read as "rich/vivid" rather than "muddy," so this
   isn't a tension with rule 1 — it's the same instinct applied to
   lightness instead of saturation.
3. **Derive `--accent-hover` by moving toward that theme's text-primary,
   not by a fixed rule of "lighter."** On dark-neutral themes, hover moves
   *lighter* (toward white text) — interaction reads as "brighter/more
   foreground." On light-neutral themes (`light`, `mint`, `grape`,
   `lemon`), hover moves *darker* (toward black text) — the conventional
   light-UI "pressed" feel. Concretely: lighten by ~12–15% toward white for
   dark-neutral themes, darken by ~20–25% toward black for the `mint`/
   `grape`/`lemon` family (they need a bigger step than `light`'s own
   ~12–15% because their accents sit at much higher saturation to begin
   with, and a small lightness step reads as barely-there against that).
4. **Compute `--accent-contrast`, don't assume white.** Check both white
   and a near-black ink against the actual accent value and pick whichever
   clears 4.5:1 — don't assume from a theme's overall lightness which one
   will win. In practice: `mint`/`grape`/`lemon`'s deep, rich accents (per
   rule 2) all land solidly on white (5.4:1–9.1:1 measured), while `dark`'s
   near-black-scale accent is the outlier that needs ink (see the known
   exception below).
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
color. Concretely in this codebase: the `mint` theme's accent is itself a
teal/green, so `--success` there is shifted toward true grass-green
(`#16692a`) rather than reading as just another shade of the accent, and
the `lemon` theme's accent is itself yellow, so `--warning` there is
shifted toward orange (`#a04008`). `grape`'s purple accent doesn't sit near
any status hue, so its status colors need no shift. `--danger` doesn't
need a per-theme shift either — red doesn't sit near any of these accent
hues — so `mint`/`grape`/`lemon` all share one vivid red (`#aa1b0e`),
tuned to clear contrast against the least forgiving of the three
`bg-base`s. Apply the same check for any future themes: if a new accent's
hue falls within roughly 30° of a status color's hue *and* they're used on
the same neutral scale, shift the status hue until they're visually
distinct at a glance, not just by looking at the hex values.

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

## The lemon hover flip

`lemon` does something no other theme does: hovering or playing a track row
in `TrackRow.svelte` inverts the row to a near-black chip with light/glowing
ink, instead of staying light like `--bg-hover`/`--bg-selected` do
everywhere else in the app. It's a deliberate "flip to dark mode on
interaction" moment — `bg-base` is about as bold as a background gets, so
inverting on interaction reads as a fun, intentional beat rather than an
inconsistency.

This is implemented as a *separate* set of tokens
(`--row-hover-bg`/`--row-hover-title-ink`/`--row-hover-num-ink`/
`--row-hover-dur-ink`/`--row-active-bg`/`--row-active-title-ink`/
`--row-active-num-ink`/`--row-active-dur-ink`, defined once in the shared
part of `theme.css`), not by overriding `--bg-hover`/`--bg-selected`
themselves. That matters: those two tokens are consumed by ten-plus other
components (buttons, popovers, the sidebar, settings, the transport bar
— `grep -rl bg-hover src` to see the current list), each pairing them with
its own fixed text color that assumes a light background. Overriding the
shared tokens directly would flip every one of those to a dark background
while their text stayed dark too, breaking contrast everywhere at once.
The dedicated `--row-*` tokens default to plain pass-throughs of the
existing text tokens (`--row-hover-title-ink: var(--text-primary)`, etc.)
— a no-op for every theme except `lemon` — so only `TrackRow.svelte`'s
markup needed to change (reading `--row-*` instead of `--bg-hover`/
`--bg-selected`/hardcoded colors directly), and every other consumer is
completely unaffected.

If this flip is extended beyond track rows later (album grid cards,
buttons, etc.), each new consumer needs the same treatment: don't
repurpose `--bg-hover`/`--bg-selected` directly, add dedicated tokens for
that consumer's background *and* every text color that sits on it, with
sensible pass-through defaults for the themes that shouldn't flip.

## Chrome tokens

`--chrome-bg`/`--chrome-hover-bg`/`--chrome-selected-bg`/`--chrome-border`/
`--chrome-text-primary`/`--chrome-text-secondary`/`--chrome-text-tertiary`
are the sidebar's and transport bar's own background and text colors — the
two chrome surfaces that are visible essentially all the time, regardless
of what's being browsed. For `mint`/`grape`/`lemon`, these are a muted
(much lower saturation than `--bg-base`), darkened tint of the theme's own
hue rather than the near-white `--bg-elevated` those themes otherwise use,
so the two most persistently-visible surfaces read as themed too, not just
the content area — an earlier pass left them washed out and themeless
besides the accent highlight. `dark`/`light` don't define these tokens at
all; they pass through to `--bg-elevated`/`--bg-hover`/`--bg-selected`/
`--border`/`--text-*` (defined once in the shared part of `theme.css`),
since `light`'s neutral chrome doesn't need its own treatment and `dark`'s
is already dark by default.

This follows the exact same pattern as the lemon hover flip above (a
dedicated token family with pass-through defaults, not repurposing shared
tokens), for the same reason: `--bg-elevated`/`--text-*` are consumed by
several *other* surfaces too (popovers, settings, device sync, the
now-playing drawer), each pairing them with assumptions that don't hold if
those tokens themselves change. `ArtistList.svelte` and `TransportBar.svelte`
(plus the `.sidebar`/`.transport-bar` wrapper rules in `+layout.svelte`)
read `--chrome-*` instead of the shared tokens; nothing else needed to
change. If this darkened-chrome treatment is extended to another
always-visible surface later, give it the same `--chrome-*` tokens rather
than inventing a third parallel set — reuse what's here unless that
surface genuinely needs different values.

Colors were derived the same way as the "dark-based colored themes" ramp
in "Tinting a colored theme's neutrals" above (muted saturation, ~20-25%,
darkened lightness) rather than the vivid, high-saturation approach used
for `--bg-base`/`--accent` — a saturated *and* dark surface read as muddy
rather than rich when tried first; muting it while keeping it dark is what
reads as "a moody, themed dark chip" instead.

## Adding a new theme, end to end

1. Pick/derive the 14 per-theme tokens above (surfaces × 4, border, text × 3,
   accent × 3, status base × 3) following the rules above — tint the
   neutrals to the new hue rather than reusing `dark`'s or `light`'s
   directly, whichever base the new theme is built on (see "Tinting a
   colored theme's neutrals").
2. Add a `:root[data-theme="yourtheme"] { … }` block to `theme.css`. Don't
   touch the shared derived-formula block.
3. Add the theme to the `Theme` union and `THEMES` list in
   `src/lib/stores/theme.svelte.ts` — the quick-access switcher (the
   palette icon next to the settings cog; there's deliberately no second
   copy of this control in Settings) reads from that single list. The
   switcher's swatch dots are the one deliberate exception to "always use a
   token": they show every theme's `--bg-base` at once, so each dot is
   keyed off a literal hex, not a `var()` (the page can only have one
   theme's tokens active at a time). Use `--bg-base`, not `--accent` — it's
   what a theme actually *looks like* at a glance, whereas `--accent` can
   mislead: `dark`'s accent is brighter than `light`'s (it needs to pop
   against a near-black background, the opposite of what "Dark"/"Light"
   implies), and for `mint`/`grape`/`lemon`, `--accent` is a deep,
   low-lightness tone chosen for text contrast, not the theme's identity
   color. Keep the swatch hex in sync by hand whenever a theme's `--bg-base`
   changes.
4. Load it in the running app and sanity-check every screen — badges,
   destructive buttons, the equalizer curve, star ratings — not just the
   toolbar and sidebar.
