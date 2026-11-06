# Bellatrix SVG artwork

Runtime icons are the independent Bellatrix SVG collection in `svg/`, registered
only by `../icons.rs`. The 65 original SVG files were moved from
`src/assets/symbols/svg/` without changing their bytes. Additional SVGs use the
same independent, currentColor-based vocabulary; category colors belong to code.

## Historical attribution

An earlier implementation used 174 outlines extracted from the vendored
StarUML 7.1.1 icon font at
`docs/ref/staruml/src/static/icon-font/icons.ttf`. That reference package
attributes copyright to MKLabs Co., Ltd. (https://staruml.io). No separate
license granting redistribution of those icons was found; attribution alone
does not grant rights.

That extracted collection and its generator are no longer present in the
runtime or the working tree. Their provenance and original notice are preserved
in Git at commit `bf268aae`, at `src/assets/uml/staruml.rs` and
`assets/staruml/NOTICE.md` relative to this crate. The independent SVG collection
does not copy those outlines and has no runtime or build-time font dependency.
