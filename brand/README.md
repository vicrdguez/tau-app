# tau logo

Drawn 2026-11. Every shape is a filled outline; no fonts are needed.

## Pieces

- **Mark** (`tau-mark*`): the written tau. Use it at 24 px and above.
- **Small mark** (`tau-mark-small*`): the same drawing with a stroke about a third heavier. Use it below 24 px.
- **Wordmark** (`tau-wordmark*`): the mark as the t, followed by a drawn "au". Keep the mark at least 24 px tall; below that, use the mark alone.
- **App icon** (`tau-icon-*`): the mark on a rounded square. Sizes 64 px and below use the small mark.
- **Favicon** (`favicon.svg`, `favicon.ico`): the small mark, dark on coral.

## Colour versions

| Suffix | Mark | Letters | Ground |
| --- | --- | --- | --- |
| `-on-dark` | `#d8d0c0` | `#d8d0c0` | dark, `#151412` |
| `-on-dark-coral` | `#e08060` | `#d8d0c0` | dark, `#151412` |
| `-on-light` | `#282418` | `#282418` | light, `#f0e8d8` |
| `-on-light-coral` | `#af482e` | `#282418` | light, `#f0e8d8` |
| (none) | `currentColor` | `currentColor` | inherits the text colour |

The coral on light grounds is the darker Ember-light coral; the dark-ground coral is too pale there.

Icons: `-dark` is `#d8d0c0` on `#252422` with a `#3e3c38` edge; `-coral` is `#151412` on `#e08060`.

## Files

- `svg/`: the sources. Backgrounds are transparent except for the icons.
- `png/`: icons at 1024 to 16 px, wordmarks at 720 by 256 and `@2x`, marks at 512 px wide.

In running text, the command and the package, the name is written `tau`.
