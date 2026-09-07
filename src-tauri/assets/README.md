# Bundled assets

These ship *inside* the binary and count against the 40 MB installer gate
(FR-59). Anything that does not belong here becomes a pack — see
[ADR 0006](../../docs/adr/0006-on-demand-packs.md).

| File | Size | Produced by | Milestone |
|---|---|---|---|
| `verse-index.bin` | ~12 MB | `scripts/build-verse-index.py`, run once, committed | M0 |
| `encoder/` | under 25 MB | On-device sentence encoder for runtime query embedding | M0 |
| `translations/kjv.pack`, `web.pack`, `asv.pack` | 4 to 6 MB each | `scripts/build-translation-pack.py` | M0 |
| `themes/default.json` | small | Hand-authored from Branding §15.2 | M3 |

Adding a file here without checking the installer size is how a 40 MB budget
becomes a 400 MB download. CI enforces the limit, but check locally first.
