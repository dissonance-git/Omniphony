# Omniphony layout geometry

This directory contains several different kinds of geometry. They are useful for different jobs and must not be conflated.

```text
CANONICAL SCENE VOCABULARY
8.1.4.4 / 17 semantic lanes
        ↓
CURRENT RENDER GEOMETRY
22-direction System-H-derived shell
        ↓
BINAURAL OUTPUT
stereo headphones
```

Reference speaker layouts such as 7.1.4 remain valuable for regression and known-scene testing, but **7.1.4 is not the Current product base**.

---

## 1. Current product shell

### `system-h-derived-22.0-upper60-grid10.yaml`

This is the Current product's 22-direction full-sphere render shell.

It sits **above** the canonical 17-lane 8.1.4.4 scene. The scene carries semantic/provenance state; this file carries internal rendering geometry.

Current product path:

```text
17-lane canonical scene
→ `system-h-derived-22.0-upper60-grid10.yaml`
→ cascaded binaural renderer
→ stereo
```

The geometry is intentionally derived from the broader System-H family while remaining tailored to Omniphony's full-sphere headphone presentation and grid-aligned height work.

Tests lock the Current shell at 22 named directions so accidental layout drift cannot silently redefine the product.

### `system-h-derived-22.0-upper60.yaml`

Related experimental/reference variant of the System-H-derived shell. Keep it available for controlled comparison, but do not treat it as the Current production geometry unless the product contract is explicitly changed and revalidated.

---

## 2. ITU/System-H references

### `reference/itu-r-bs2051-system-h-22.0.yaml`

Reference-oriented System-H geometry used for comparison and standards-facing work.

### `itu-r-bs2051-system-h-22.0.yaml`

Repository working geometry in the System-H family. It remains useful as a known-layout surface independent of the Current shell.

The existence of these files does not make the Current 22-direction shell a literal standards speaker layout. Current uses System-H-derived geometry as a rendering lattice.

---

## 3. Immersive regression/reference layouts

These layouts are useful for deterministic known-scene tests, speaker rendering and rich-input development.

### `5.1.2.yaml`

5.1 bed plus two height channels.

### `7.1.2.yaml`

7.1 bed plus two height channels.

### `7.1.4.yaml`

7.1 bed plus four height channels.

This is an important regression/reference layout, but it is **not** the foundational Current scene. Current's semantic base is the 17-lane 8.1.4.4 vocabulary described in the root documentation.

### `9.1.6.yaml`

9.1 bed plus six height channels.

---

## 4. Legacy layouts

The `legacy/` directory retains conventional no-height or older bridge-era layouts:

```text
2.0
2.1
4.0
4.1
5.0
5.1
6.1
7.1
```

They remain useful for compatibility, calibration and regression work. They do not define the Current headphone product architecture.

---

## 5. Coordinate convention

Checked-in layouts use Cartesian coordinates:

```text
x: right positive, left negative
y: front positive, rear negative
z: up positive, down negative
```

Repository conventions:

- `coord_mode: "cartesian"` for checked-in speaker entries;
- normalized coordinates where appropriate;
- LFE marked `spatialize: false`;
- unique speaker names;
- symmetry used where the intended geometry is symmetric.

Polar coordinates remain parser-supported for external/custom layouts.

---

## 6. Scene semantics are not stored here

A layout file describes render/speaker geometry. It does **not** by itself encode whether a lane is authored, derived or empty.

That provenance belongs to the scene/source contract:

```text
AUTHORED
DERIVED
EMPTY
```

For example, stereo-derived Current currently leaves:

```text
C LFE Cb Bfl Bfr Bbl Bbr
```

EMPTY even though the canonical scene knows those positions exist.

Do not activate a canonical lane merely because some layout contains a nearby speaker direction.

---

## 7. Known scene versus Current shell

Use known layouts to test renderer behavior independently from stereo inference.

```text
KNOWN-SCENE TEST
7.1.4 fixture
→ known geometry
→ renderer validation

CURRENT PRODUCT
stereo evidence
→ canonical 8.1.4.4 scene
→ 22-direction Current shell
→ binaural
```

Both are valuable. They answer different questions.

---

## 8. Loading a layout

For generic renderer/CLI work, layouts may be loaded through the existing speaker-layout interfaces. Example:

```bash
orender render --enable-vbap --speaker-layout layouts/7.1.4.yaml input.bin
```

The Current product shell is embedded by the product rendering path and should not require users to choose a speaker preset for normal headphone listening.

---

## 9. Custom layout requirements

For custom VBAP-capable layouts:

1. use at least three spatialized speakers;
2. keep names unique;
3. keep the coordinate system explicit;
4. mark LFE non-spatialized;
5. avoid degenerate or duplicate positions;
6. validate the geometry before using it as a listening reference.

Custom layouts are laboratory/rendering inputs. They do not alter the canonical scene vocabulary unless the product contract itself changes.

---

## 10. Validation law

Changes to Current geometry should trigger both geometry and binaural tests.

At minimum, Current validation should prove:

```text
canonical scene remains 17 lanes
Current shell remains 22 directions
EMPTY stereo lanes remain empty
scene reaches binaural renderer
output remains finite stereo
```

The wide DSP validation workflow and `orender_engine/tests/current_scene_geometry.rs` are the primary product-level guards for this boundary.

---

## Reference standards

Relevant standards families include:

- ITU-R BS.775 for conventional multichannel stereophony;
- ITU-R BS.2051 for advanced sound-system layouts;
- SMPTE ST 2098-2 for immersive-audio bitstream concepts.

Standards references are used to anchor geometry and terminology. The Current shell remains an Omniphony rendering lattice unless explicitly documented otherwise.
