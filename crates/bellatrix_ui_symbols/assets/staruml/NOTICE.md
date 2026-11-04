# StarUML Explorer icons

The 174 outlines in `src/staruml.rs` are extracted without shape changes from the
StarUML 7.1.1 icon font already present in this repository at
`crates/bellatrix_design_system/docs/ref/staruml/src/static/icon-font/icons.ttf`.
Names/codepoints come from the adjacent `icons.css`; semantic color categories
come from `src/static/styles/icon-colors.less`. The source package attributes
copyright to MKLabs Co., Ltd. (https://staruml.io).

No separate license granting redistribution of these icons was found in the
vendored reference. This notice preserves provenance; it does not grant rights.
Confirm the applicable license with the rights holder before distributing them.

To regenerate, run `python crates/bellatrix_ui_symbols/assets/staruml/generate.py`
from the repository, with Python fontTools installed. No runtime font dependency
or build-time generator is required. GPUI renders the outlines as masks, with
category colors chosen for the current light or dark background. Selection keeps
the category color, so it never erases the icon's semantic meaning.
