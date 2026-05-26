# Arch/CachyOS packaging

PKGBUILDs for the mpv-omniphony integration's runtime libraries. The mpv package
itself (`mpv-omniphony`) lives in the separate `mpv-omniphony` repo and depends on
`liborender>=0.1`.

| Package                   | Builds from            | License      | Installs |
|---------------------------|------------------------|--------------|----------|
| `liborender`              | this repo's workspace  | GPL-3.0-only | `liborender.so*`, `orender.h`, `orender.pc`, layouts |
| `omniphony-truehd-bridge` | sibling `harletty-bridge` | Apache-2.0 | `/usr/lib/orender/truehd_bridge.so` |

`liborender` depends on `omniphony-truehd-bridge`: the decoder is a runtime
`dlopen` plugin (the `*_bridge.so` pattern), so it is packaged separately
(different repo, different license) and pulled in as a dependency.

## Install layout

```
/usr/lib/liborender.so.0.1.0     # real cdylib (DT_SONAME = liborender.so.0)
/usr/lib/liborender.so.0         # → liborender.so.0.1.0
/usr/lib/liborender.so           # → liborender.so.0      (dev/link symlink)
/usr/include/orender.h
/usr/lib/pkgconfig/orender.pc
/usr/share/orender/layouts/*.yaml  # virtual-bed fallback looks here
/usr/lib/orender/truehd_bridge.so  # the decoder bridge plugin
```

The bridge has no exe-relative search when hosted in mpv, so point mpv at it
explicitly:

```
mpv --ad=orender \
    --ad-orender-bridge-path=/usr/lib/orender/truehd_bridge.so \
    --ad-orender-config=/path/to/omniphony.yaml  film.atmos.mkv
```

## Building

These fetch pinned release tarballs — no checkout layout needed:

- `omniphony-truehd-bridge` 0.6.0 ← harletty-bridge `v0.6.0`, plus the matching
  Omniphony `liborender-v0.1.0` source for its workspace path-deps
  (`bridge_api`/`spdif`/`sys`).
- `liborender` 0.2.0 ← Omniphony `liborender-v0.2.0`.

Build the bridge first (`liborender` depends on it):

```sh
cd omniphony-truehd-bridge && makepkg -si
cd ../liborender           && makepkg -si
```

Then the mpv package from the separate `mpv-omniphony` repo (depends on
`liborender>=0.1`).

**Bumping a release:** retag the source repo(s), then refresh the version vars
(`pkgver`, and `_omniver` in the bridge) and `sha256sums` (`updpkgsums` or
`makepkg -g`).

### Clean-room build

Both build cleanly from the fetched tarballs (validated with `makepkg`, no
`$srcdir` leakage thanks to `--remap-path-prefix`). For a fully isolated build
before publishing, use a chroot (bridge first):

```sh
makechrootpkg -c -r "$CHROOT"   # run in each package dir
```

## Verify

```sh
PKG_CONFIG_PATH=<pkgdir>/usr/lib/pkgconfig pkg-config --cflags --libs orender
# -> -I<...>/usr/include -L<...>/usr/lib -lorender
```
