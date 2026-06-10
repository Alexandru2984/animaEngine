# Icon variants — animaEngine 0.2.0

Three exploratory directions for the app icon, decided during Faza A.2.
Open each `.svg` in your favourite viewer (Inkscape, Eye of GNOME,
Files preview pane) at 256×256 *and* 24×24 to see how each scales.

| File | Direction | Vibe |
|------|-----------|------|
| `v1-ghost-mascot.svg` | Friendly mascot (leans on existing demo asset) | warm, character-driven, memorable |
| `v2-orbit.svg`        | Abstract motion / multi-entity engine          | technical, serious, "product" |
| `v3-frame-stack.svg`  | Literal frame-by-frame metaphor + play triangle | clear, accessible, recognizable in launchers |

All three use the design-system accent palette
([docs/design-system.md §1](../../docs/design-system.md#1-color-system))
so the chosen icon stays coherent with the in-app UI on day one.

The final pick replaces `build/AppDir/usr/share/icons/hicolor/scalable/apps/anima-engine.svg`
and gets exported to PNG at 16/24/32/48/64/128/256 px for the `.deb` /
AppImage / Flatpak hicolor theme tree (handled in the existing
`make install` rules).

## ⚠ File-format constraint (launcher icons go blank otherwise)

Every SVG here **must start with the literal bytes `<svg`** — no XML
prolog (`<?xml …?>`), no leading comment, no BOM. gdk-pixbuf detects
the format by sniffing the first bytes (librsvg's loader signature is
anchored at the file start), and GNOME Shell renders launcher icons
through gdk-pixbuf. An icon with a prolog parses fine in Inkscape,
Eye of GNOME and `rsvg-convert` — and then renders as a **blank tile**
in the Shell app grid. Keep design notes in a comment *inside* the
`<svg>` element instead. (Root cause of the missing launcher icon
from 0.2.0 through 0.5.4.)
