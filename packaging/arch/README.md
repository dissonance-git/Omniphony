# Arch/CachyOS packaging

PKGBUILDs for the mpv-orender integration's runtime libraries. The mpv package
itself (`mpv-orender`) lives in the separate `mpv-orender` repo and depends on
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

## Building (local/dev)

These are **local/dev** PKGBUILDs: they build in place from the checkouts on
disk (no network `source=()`), so build them where the sibling repos live:

```
spatial-renderer/
├── Omniphony/         # this repo (PKGBUILDs under packaging/arch/)
└── harletty-bridge/   # the decoder bridge (sibling)
```

```sh
cd Omniphony/packaging/arch/omniphony-truehd-bridge && makepkg -f
cd ../liborender                                     && makepkg -fd   # -d: bridge dep not installed yet
```

Override the bridge source location with `BRIDGE_SRC=/path/to/harletty-bridge`.

For a **release** package, replace the in-place build with a versioned git or
tarball `source=()` plus real `sha256sums`, and drop the `-d`/`BRIDGE_SRC`
shortcuts.

## Verify

```sh
PKG_CONFIG_PATH=<pkgdir>/usr/lib/pkgconfig pkg-config --cflags --libs orender
# -> -I<...>/usr/include -L<...>/usr/lib -lorender
```
