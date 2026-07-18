# MorpheOS Voice visual direction

## Direction

MorpheOS Voice should look like a focused MorpheOS product, not a separate AI startup. The verified parent system at `morpheos.net` is restrained, practical and high-contrast. The interim Voice identity therefore uses the parent `M` mark and type system, then adds a small voice/cursor signal as the product distinction.

This is an implementation-ready interim system. It deliberately avoids inventing a complex logo while product identity, signing and trademark work remain open.

## Parent system carried into Voice

| Token | Value | Use |
|---|---|---|
| Background | `#10110f` | Primary website and desktop surface |
| Raised background | `#171812` | Panels, tray/status cards |
| Soft background | `#202116` | Secondary controls |
| Paper | `#f2efe4` | Primary text and light icon field |
| Muted paper | `#d2cbb8` | Secondary text |
| Muted | `#b8b09d` | Supporting metadata |
| Line | `#373829` | Standard borders |
| Strong line | `#5f6144` | Emphasised borders and inactive signal |
| Signal green | `#b9f27c` | Ready/success and primary action |
| Signal ink | `#16220d` | Text on signal green |
| Brass | `#d3a94f` | Processing/attention support |
| Rust | `#d66f3f` | Error/destructive support |
| Focus blue | `#8bd7ff` | Keyboard focus indicator |

Typography:

- **Body/display:** IBM Plex Sans, with Aptos and Segoe UI fallbacks.
- **Technical/status:** IBM Plex Mono, with SFMono-Regular and Consolas fallbacks.
- Do not bundle font files in the application during this pass. Desktop surfaces use installed/system fallbacks; the public MorpheOS site already loads the IBM Plex family.

## Logo and wordmark usage

### Parent mark

Use the existing MorpheOS `M` mark: a square, high-contrast container with a monospaced `M`. It identifies the parent company and is appropriate in the website header.

### Product lockup

The standard horizontal lockup is:

> `[M mark] MorpheOS`<br>
> `Voice`

At narrow sizes, use `[M mark] MorpheOS Voice` on one line. `Voice` must remain a product identifier, not replace MorpheOS.

Do not abbreviate the public product to `MV` or `Voice AI`.

### App icon

The interim app icon combines:

- the parent paper-on-dark square;
- a single `M` anchor;
- three restrained vertical voice bars ending at a cursor baseline;
- signal green only as a functional accent.

It must remain recognisable at 16 px. No small text beyond the `M`; no gradients, glows, faces, heads, stars, pills or imitation OpenAI/Whisper marks.

`[NEEDS TYLER]` Final product-mark approval before paid trademark/design work or platform-store submission.

## Functional state language

The product signal is functional, not decorative:

- **Ready:** paper/neutral with a small green cursor.
- **Arming:** brass, restrained pulse.
- **Listening:** green voice bars with a clear “Listening” text label.
- **Processing:** brass progress treatment with “Processing”.
- **Inserted:** green confirmation with “Inserted”.
- **Copied:** focus blue or neutral confirmation with “Copied”.
- **Needs attention:** rust/error with a concrete recovery action.
- **Cancelled:** muted neutral.

Never rely on colour alone. Every state needs visible text and, where practical, an icon/form change.

## Minimum sizes and clear space

- App/tray glyph: minimum 16 × 16 px; test at 16, 20, 24 and 32 px.
- Parent header mark: minimum 28 × 28 px; preferred 34 × 34 px.
- Horizontal product lockup: minimum 132 px wide in digital layouts.
- Keep clear space of at least one quarter of the mark width around the icon/lockup.
- Avoid placing the mark on detailed imagery or low-contrast colour.

## Light and dark use

Dark is the primary application treatment because it matches the parent MorpheOS product system and keeps the recording signal calm.

- **Dark:** background `#10110f`, mark field `#f2efe4`, ink `#10110f`, signal `#b9f27c`.
- **Light:** background `#f2efe4`, mark field `#10110f`, ink `#f2efe4`, signal accent `#426b24` or another WCAG-proved dark green rather than the light signal green for small text.
- Single-colour marks must use pure foreground/background contrast and retain the voice/cursor silhouette.

## Accessibility requirements

- Normal text must meet WCAG 2.2 AA contrast of at least 4.5:1; large text at least 3:1.
- Controls, focus rings and meaningful graphical objects must meet at least 3:1 against adjacent colours.
- Keyboard focus uses a visible 3 px `#8bd7ff` outline with offset.
- Reduced-motion preferences must stop pulsing/wave animation.
- Recording must be announced through an `aria-live` status without exposing transcript content.
- The Signal window must remain non-focusable so it cannot steal the insertion target.
- Never place essential status only in a tray icon that a screen reader cannot explain.

## Prohibited treatments

- Morpheus from *The Matrix* or *The Sandman*.
- Red-pill or blue-pill imagery.
- OpenAI knot, Whisper waveform or other third-party logo imitation.
- Humanoid AI heads, brains, robots or assistant avatars.
- Neon cyan star fields, excessive gradients, glassmorphism or “future AI” decoration.
- Audio waveforms that resemble real captured speech when no audio visualisation exists.
- Animated microphone treatment that claims capture before the backend confirms it.
- “OS” legacy initials on new public assets.

## Asset inventory

### Existing tracked assets to replace in content, not path

Paths remain stable where packaging depends on them:

| File | Current dimensions/form | Target treatment |
|---|---|---|
| `website/favicon.svg` | SVG, 128 × 128 viewBox | MorpheOS Voice `M` + voice/cursor mark |
| `packaging/linux/oswispa.svg` | SVG, 128 × 128 viewBox | Same interim icon; legacy filename retained for package compatibility |
| `desktop/src-tauri/icons/icon.svg` | SVG, 128 viewBox | Same icon at native vector size |
| `desktop/src-tauri/icons/icon.png` | PNG, 512 × 512 RGBA | Regenerated Tauri raster master |
| `desktop/src-tauri/icons/icon.ico` | Windows multi-resolution ICO | Regenerated with 16, 24, 32, 48, 64, 128 and 256 px entries |

### New brand assets

| File | Dimensions/form | Purpose |
|---|---|---|
| `website/morpheos-voice-mark.svg` | SVG, 128 × 128 viewBox | Standalone product mark for `/voice` |
| `website/morpheos-voice-lockup.svg` | SVG, 640 × 160 viewBox | Product lockup for documentation/share use |
| `website/morpheos-voice-social.png` | PNG, 1200 × 630 | Open Graph/social preview for the free-product page |

No marketing screenshot is currently approved. `docs/rebrand/evidence/linux-controlled-e2e.png` is test evidence from a controlled rebranded Linux dictation run, not promotional artwork.

## Logo exploration decision

Two symbol-only SVG explorations were generated with the saved Replicate/Recraft route after Tyler authorised credential use. Both ignored hard constraints: one produced literal microphones and the other added a person plus colour-code text. They were rejected and are not tracked. The smaller project-authored geometric `M` + voice/cursor mark remains the interim asset because it is clearer at tray/icon sizes and does not borrow another product's visual language.

## Relationship to the free-product page

The `/voice` page should inherit the MorpheOS header rhythm and palette, then give the product its own simple demonstration rail. It may link quietly to the wider MorpheOS business after the primary download/open-source journey. It must not inherit IntelligenceOS pricing banners, imply MorpheOS Voice is a trial or place a paid CTA ahead of the free download.

## Validation checklist

- Inspect the icon at 16, 20, 24, 32, 64, 128, 256 and 512 px.
- Check Windows ICO contains the required sizes.
- Check PNG alpha and colour profile are stable.
- Run automated contrast checks on the website and desktop UI.
- Verify forced-colours and reduced-motion behaviour.
- Capture light/dark screenshots only from the running rebranded build.
- Confirm `aria-label` text says “MorpheOS Voice”, not the legacy name.
