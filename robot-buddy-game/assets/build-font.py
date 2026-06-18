#!/usr/bin/env python3
"""Regenerate `unifont-subset.ttf` from the system GNU Unifont .otf files.

Subsets GNU Unifont to the Unicode ranges this game renders, converts the CFF
outlines to TrueType `glyf` (so fontdue rasterizes them with no CFF path), and
merges the BMP + upper-plane subsets into a single embeddable font.

Prereqs:
    sudo apt-get install -y fonts-unifont
    pip install fonttools otf2ttf

Usage:
    python3 robot-buddy-game/assets/build-font.py
"""

import os
import subprocess
import sys
import tempfile

from fontTools import merge
from fontTools.ttLib import TTFont

BMP = "/usr/share/fonts/opentype/unifont/unifont.otf"          # Plane 0
UPPER = "/usr/share/fonts/opentype/unifont/unifont_upper.otf"  # Planes 1+
OUT = os.path.join(os.path.dirname(__file__), "unifont-subset.ttf")

# Unicode ranges we want to guarantee render. Kept tight to stay small, but
# generous enough across the emoji blocks that future dialogue won't show tofu.
RANGES = [
    "U+0020-00FF",    # Basic Latin + Latin-1 Supplement (incl. × ÷)
    "U+2000-206F",    # General Punctuation (dashes, curly quotes, …)
    "U+2070-209F",    # Super/subscripts
    "U+2190-21FF",    # Arrows
    "U+2200-22FF",    # Mathematical Operators (− U+2212)
    "U+2500-257F",    # Box Drawing
    "U+2580-259F",    # Block Elements
    "U+25A0-25FF",    # Geometric Shapes
    "U+2600-26FF",    # Miscellaneous Symbols (★ U+2605)
    "U+2700-27BF",    # Dingbats
    "U+2B00-2BFF",    # Misc Symbols and Arrows (⭐ U+2B50, ⬆ ⬇)
    "U+1F300-1F5FF",  # Misc Symbols and Pictographs
    "U+1F600-1F64F",  # Emoticons
    "U+1F680-1F6FF",  # Transport and Map (🚀)
    "U+1F900-1F9FF",  # Supplemental Symbols and Pictographs (🤖 U+1F916)
    "U+1FA70-1FAFF",  # Symbols and Pictographs Extended-A
]
UNICODES = ",".join(RANGES)

# Glyphs we actually draw today — fail loudly if any goes missing.
REQUIRED = {
    0x2212: "−", 0x00D7: "×", 0x00F7: "÷", 0x2605: "★", 0x2B50: "⭐",
    0x1F916: "🤖", 0x1F680: "🚀", 0x1F31F: "🌟", 0x1F36D: "🍭", 0x1F4CD: "📍",
    0x0041: "A", 0x0039: "9", 0x0024: "$", 0x0023: "#", 0x0020: "space",
}


def run(*args):
    subprocess.run(args, check=True, stdout=subprocess.DEVNULL)


def main():
    for src in (BMP, UPPER):
        if not os.path.exists(src):
            sys.exit(f"missing {src} — install the 'fonts-unifont' package")

    with tempfile.TemporaryDirectory() as tmp:
        parts = []
        for name, src in (("bmp", BMP), ("upper", UPPER)):
            otf = os.path.join(tmp, name + ".otf")
            ttf = os.path.join(tmp, name + ".ttf")
            # Subset (ignore codepoints this plane doesn't have).
            run(sys.executable, "-m", "fontTools.subset", src,
                f"--unicodes={UNICODES}", "--ignore-missing-unicodes",
                "--drop-tables+=DSIG", f"--output-file={otf}")
            # CFF outlines -> TrueType glyf, so fontdue rasterizes directly and
            # the two halves can be merged (CFF merging is unsupported).
            run("otf2ttf", "-o", ttf, otf)
            parts.append(ttf)

        merged = merge.Merger().merge(parts)
        merged.save(OUT)

    cmap = TTFont(OUT).getBestCmap()
    missing = [f"U+{cp:04X} {ch}" for cp, ch in REQUIRED.items() if cp not in cmap]
    if missing:
        sys.exit("required glyphs missing from output: " + ", ".join(missing))

    print(f"wrote {OUT} ({os.path.getsize(OUT)} bytes, "
          f"{len(TTFont(OUT).getGlyphOrder())} glyphs) — all required glyphs present")


if __name__ == "__main__":
    main()
