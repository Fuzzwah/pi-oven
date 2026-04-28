# Bundled monospace font

Drop a single monospace `.ttf` (or `.otf`) file in this directory — for
example, a [JetBrains Mono Nerd Font](https://www.nerdfonts.com/font-downloads)
regular-weight subset. The renderer's `Painter::new` looks up any `.ttf` files
here at startup and registers them with `glyphon::FontSystem`. If no font is
present, the renderer falls back to whichever monospace font the OS exposes
through its system font database.

The font is intentionally not checked in to this repo — pick one with a license
that allows redistribution before bundling it for release.
