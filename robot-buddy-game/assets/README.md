# Bundled font

## `unifont-subset.ttf`

The glyph font embedded in the WASM binary (`src/text.rs` `include_bytes!`s it).
It exists so every glyph we draw — Latin text, the math operators `− × ÷`, the
star `★`, and emoji like `🤖 🚀 🌟 🍭 📍` — renders instead of falling back to
tofu. macroquad's built-in font (`ProggyClean`) covers none of those.

### What it is

A subset of **[GNU Unifont](https://unifoundry.com/unifont/)** — a 16px
bitmap-style pixel font that covers (nearly) all of Unicode. The pixel look
sits naturally next to the retro tile art, and it's one of the very few fonts
that carries glyphs for text, math symbols, **and** emoji in a single family.

macroquad rasterizes glyphs to a monochrome alpha mask (via fontdue), so the
emoji render as single-color silhouettes tinted by the draw color — color
emoji are not possible in this engine.

### How it was built

Source files ship with the Debian `fonts-unifont` package:

- `/usr/share/fonts/opentype/unifont/unifont.otf` — Plane 0 (BMP): text, `− × ÷ ★ ⭐`
- `/usr/share/fonts/opentype/unifont/unifont_upper.otf` — Planes 1+: the
  astral-plane emoji (`🤖 🚀 🌟 🍭 …`)

`build-font.py` subsets each to the Unicode ranges we render, converts the CFF
outlines to TrueType `glyf` (so fontdue can rasterize them directly), and
merges the two into one file. To regenerate:

```bash
sudo apt-get install -y fonts-unifont
pip install fonttools otf2ttf
python3 robot-buddy-game/assets/build-font.py
```

### License

GNU Unifont is distributed under the **GNU GPL version 2 or later, with the GNU
Font Embedding Exception**, and is additionally dual-licensed under the **SIL
Open Font License v1.1**. The embedding exception reads, in part:

> As a special exception, if you create a document which uses this font, and
> embed this font or unaltered portions of this font into the document, this
> font does not by itself cause the resulting document to be covered by the GNU
> General Public License.

Embedding the (subset, otherwise unaltered) glyphs into this game binary is
exactly that case, so bundling it places no GPL obligation on the game's own
source. Copyright © 1998–2019 the GNU Unifont authors (Paul Hardy et al.).
