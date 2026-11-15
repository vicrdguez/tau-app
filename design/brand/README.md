# Brand files used by the app

Copied from the source folder `~/dev/tau-brand/` (SVG sources in `svg/`, every PNG size in `png/`, a preview in `preview.png`). Every shape is a filled outline; no font is needed. The name is written `tau` in prose and commands; `τau` is the logo only.

| File | Where it goes |
|---|---|
| `tau-icon-small-coral.svg` | the sidebar's avatar, 24 px, beside `tau` and the server line, in every theme; also the app icon source at 64 px and below |
| `tau-wordmark-on-dark-coral.svg` | the start page and Connect, 44 px tall, in Ember and Ember Soft: a coral τ (`#e08060`) and an `fg` "au" (`#d8d0c0`) |
| `tau-wordmark-on-light-coral.svg` | the same in Ember Light: the darker Light coral τ (`#af482e`) and an `fg` "au" (`#282418`) |
| `tau-icon-coral.svg` | the Dock and app icon source, 128 px and above; the Dock badge carries the `◆` count |
| `tau-icon-dark.svg` | an alternative app icon, not the default |
| `icon/tau-icon-coral-{16,32,64,128,256,512,1024}.png` | the macOS icon set: `icon_16x16` (16), `icon_16x16@2x` and `icon_32x32` (32), `icon_32x32@2x` (64), `icon_128x128` (128), `icon_128x128@2x` and `icon_256x256` (256), `icon_256x256@2x` and `icon_512x512` (512), `icon_512x512@2x` (1024) |

Inside the window the brand is coral in exactly two places, the avatar and the wordmark's τ (`reference/visual-language.md` §11). The client draws the wordmark files as they are and never tints them. The ink marks of the previous pass (`tau-mark-small`, `tau-mark`, `tau-wordmark`) are no longer used: Home has no tab, the Server float no mark, and Connect takes the coral wordmark. They remain in `~/dev/tau-brand/svg/`.
