# Build scripts

These run on a developer machine, never inside the app. Each is a one-time or
release-time job whose output is committed or uploaded to the Pack CDN.

| Script | When | Output |
|---|---|---|
| `build-verse-index.py` | Once (M0) | `src-tauri/assets/verse-index.bin`, ~12 MB, committed |
| `build-translation-pack.py` | Per translation (M0, M6) | A compressed pack + SHA-256 for the CDN |
| `publish-packs.sh` | Each pack release | Uploads packs and a signed `packs-manifest.json` |

## build-verse-index.py

Embeds all 31,102 verses with OpenAI `text-embedding-3-small`, reduces to 384
dimensions, quantizes to int8, and writes a flat binary the Rust side memory-maps
for brute-force cosine search (PRD §15.4, FR-15). Costs about $2 to run. The
API key is read from `OPENAI_API_KEY`; this is the only place OpenAI is used and
it is never called from the shipped app.

Requires: `python 3.11+`, `openai`, `numpy`, `scikit-learn`.

## build-translation-pack.py

Pulls a translation from API.Bible, normalizes it into the `bible_verses` row
shape, compresses it, and emits the pack plus its checksum. Commercial
translations require the license to be in place first (M6).

## publish-packs.sh

Uploads built packs to object storage, regenerates `packs-manifest.json` with
real sizes and checksums, signs the manifest, and invalidates the CDN cache.
