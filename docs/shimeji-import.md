# Shimeji pack import (U.0 research → U.3 implementation)

Why this feature: Shimeji ("desktop mascots") has a large community
with thousands of freely shared character packs, and no maintained
Linux engine — the original and the -ee fork are Java, effectively
Windows-first, and abandoned. Importing the packs (not the engine
semantics) gives animaEngine an instant content ecosystem.

This document fixes the subset we map, the caps we enforce, and what
we deliberately drop. The importer (U.3) is built against this doc
plus golden-file fixtures; where the wild disagrees with this doc,
the fixtures win and this doc gets updated.

## Pack anatomy (shimeji-ee convention)

```
PackName/
├── img/
│   ├── shime1.png … shimeN.png     numbered sprite frames,
│   │                               transparent PNG, typically 128px
│   └── icon.png                    optional tray icon (ignored)
└── conf/
    ├── actions.xml                 action definitions
    └── behaviors.xml               behavior selection table
```

Variations seen in the wild (fixtures must cover):

- `img/` sprites in subfolders per costume; alternate naming
  (`shime1a.png`); uppercase extensions.
- `conf/` files named `actions.xml`/`behaviors.xml` but also
  `行動.xml`/`動作.xml` (Japanese originals) — match by root element
  (`<Mascot>`), not filename.
- XML in UTF-8 with or without BOM; some packs are Shift-JIS
  (declared in the XML prolog) — **reject non-UTF-8 with a clear
  skip reason** rather than mis-decode (U.3 cap list).

## Format semantics (the parts we care about)

`actions.xml` defines named actions of types:

- `Stay` — hold a pose set in place (≈ our `Idle`).
- `Move` — pose sequence with a velocity, until a border is reached
  (≈ our `Walk` with direction + edge bounce).
- `Animate` — fixed-duration pose sequence (≈ a one-shot animation;
  v1 maps it to `Idle`-family loops).
- `Sequence` / `Select` — composition/choice nodes referencing other
  actions (flattened: we follow references one level to find pose
  sequences; deeper nesting → skip with reason).
- `Embedded` — references to Java classes (`SitAndFaceMouse`,
  `ClimbWall`…) — **dropped**, no mapping target.

Each pose: `Image` (sprite path), `ImageAnchor`, `Velocity`,
`Duration` (in engine ticks ≈ 40 ms each — converted to per-frame
delays in ms).

`behaviors.xml` weights actions into behaviors with `Frequency`,
conditions (`mascot.y > …` JavaScript-ish expressions — dropped) and
"next behavior" chains (dropped; our behavior engine drives state).

## Mapping table (v1)

| Shimeji | animaEngine | Notes |
|---|---|---|
| `Stay` poses (Stand/Sit) | `AnimationSet::Idle` | first Stay action wins; others become alternates post-1.0 |
| `Move` walk poses | `AnimationSet::Walk` + `Behavior::WalkAround` | velocity → walk speed; horizontal flip handled by the engine (U.2) |
| `Fall` action poses | `AnimationSet::Fall` | played while gravity is active |
| `Dragged`/`Pinched` poses | `AnimationSet::Drag` | played while the user drags |
| `ClimbWall`/`ClimbCeiling`, `Embedded` | — dropped | needs window-geometry awareness; post-1.0 at best |
| multi-mascot interactions (`Broadcast`/`ScanMove`) | — dropped | out of scope |
| behavior frequency table | — dropped | our behaviors are user-driven |

One pack → one `CharacterConfig` with a 4-state `AnimationSet`
(missing states fall back to `Idle` — U.2 rule). Sprites are copied
(not referenced) into the asset library under
`<library>/imported/<pack-slug>/`, so deleting the source folder
doesn't orphan the scene.

## Security caps (enforced in U.3, fuzzed in W.4)

XML side — parser is `quick-xml` with:

- DTD/doctype **rejected** (no entity expansion, no external
  entities — the classic XML bombs die here);
- max document size 2 MiB per file; max element depth 32; max
  attribute length 4 KiB;
- image references canonicalised under the pack root
  (`resolve_library_asset` pattern) — `../` or absolute paths in
  `Image=` attributes → skip entry with reason.

Image side — the existing asset pipeline applies unchanged:
`MAX_IMAGE_DIM`, `MAX_DECODED_ASSET_BYTES`, aggregate memory budget,
extension whitelist. Pack-level additions:

- max 512 sprite files per pack (a pack averages 40–50);
- max 64 MiB on-disk pack size (stat-based, before any decode);
- import is all-or-nothing per *character*, best-effort per *pack*:
  a malformed action skips that character with a reason in the
  import report, never aborts the whole pack.

## Licensing note

Packs are user content with wildly varying licenses (most are
fan-made, many derived from copyrighted characters). animaEngine
never bundles, fetches or recommends packs; the importer only reads
what the user already has. Test fixtures are **generated
programmatically** inside `src/shimeji/mod.rs` tests (tiny PNGs via
the `image` crate + a constant `actions.xml`) — no binary fixtures
in the repo at all.

## Open questions → resolved by fixtures in U.3

1. Tick duration: 40 ms is the -ee default; some forks use 25 ms.
   Fixture decision: read `Duration` as ticks × 40 ms, clamp
   per-frame delay to [20 ms, 10 s].
2. Anchor semantics: Shimeji anchors poses at the *foot* center; our
   sprites anchor at top-left. The importer normalises by the max
   sprite bounding box per state so frames don't jitter.
3. `ImageRight` (pre-flipped right-facing sprites): v1 ignores them
   and lets the engine mirror (U.2); revisit if fixtures show
   asymmetric art where mirroring looks wrong.
