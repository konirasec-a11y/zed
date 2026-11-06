"""Extract vendored StarUML outlines; requires fontTools, not used by the build."""

from pathlib import Path
import re

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.ttLib import TTFont

root = Path(__file__).resolve().parents[4]
source = root / "crates/bellatrix_mde_ui/docs/ref/staruml/src/static"
font = TTFont(source / "icon-font/icons.ttf")
glyphs = font.getGlyphSet()
character_map = font.getBestCmap()
colors = dict(re.findall(
    r"\.icon-(\w+)\s*\{ color: @icon-color-(\w+)",
    (source / "styles/icon-colors.less").read_text(),
))
icons = re.findall(
    r'\.icon-((?:UML\w+)|Project):before\s*\{\s*content:"\\([0-9a-f]+)"',
    (source / "icon-font/icons.css").read_text(),
)
lines = [
    "//! Outlines extracted from the vendored StarUML icon font. See assets/staruml/NOTICE.md.",
    "",
    "#[derive(Clone, Copy)]",
    "pub enum ExplorerIconCategory {",
    "    Project,",
    "    Element,",
    "    Relation,",
    "    Diagram,",
    "}",
    "",
    "pub fn explorer_glyph(name: &str) -> Option<(&'static [u8], ExplorerIconCategory)> {",
    "    Some(match name {",
]
for name, codepoint in icons:
    pen = SVGPathPen(glyphs)
    glyphs[character_map[int(codepoint, 16)]].draw(pen)
    units = font["head"].unitsPerEm
    ascent = font["hhea"].ascent
    svg = (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {units} {units}">'
        f'<path transform="translate(0 {ascent}) scale(1 -1)" d="{pen.getCommands()}"/></svg>'
    )
    category = colors.get(name, "element").title()
    lines.append(
        f'        "{name.removeprefix("UML")}" => '
        f'(br#"{svg}"#, ExplorerIconCategory::{category}),'
    )
lines.extend(["        _ => return None,", "    })", "}"])
(root / "crates/bellatrix_mde_ui/assets/staruml/legacy-outlines.rs").write_text("\n".join(lines) + "\n")
print(f"Extracted {len(icons)} icons.")
