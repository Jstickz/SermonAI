# Staged translation packs

Packs here are **built and validated but not shipped**. This directory sits
outside `src-tauri/assets`, which is what the Tauri bundle globs
(`"resources": ["assets/**/*"]`), so nothing in here reaches an installer.

## What is staged

### `translations/kjv.jsonl.gz` — King James Version

A fallback, prepared because the KJV is the translation most of our target
churches preach from and it is **not licensed to our YouVersion app key**. The
only King James in YouVersion's 1,485-version catalog is a Thai one (id 175).

- **Source:** ebible.org's `eng_kjv` via [helloao.org](https://bible.helloao.org)
  (the Free Use Bible API). 1769 Blayney revision.
- **Not** wldeh/bible-api: its chapter files duplicate every verse, returning
  62 verses for Genesis 1 instead of 31.
- **Provenance note:** the same edition Project Gutenberg distributes as eBook
  #10. Gutenberg's copy is plain prose and would need re-versifying, so the
  machine-readable ebible.org text is used instead.
- 31,102 verses across 66 books, cross-checked against the source's own
  per-book verse counts.

Rebuild with `python scripts/build-kjv-fallback.py`.

## Before enabling this

**Check the UK position.** The KJV is public domain in the United States. In
the United Kingdom it is under perpetual Crown copyright, administered by
Cambridge University Press under letters patent, which permits quoting
passages but is not the same freedom as public domain. PRD §4.2 lists the UK
among target markets, so this needs a deliberate decision rather than an
assumption.

If YouVersion enables English KJV for our app key, prefer their copy: it comes
with a publisher-supplied attribution string, which is what §15.2 requires, and
keeps every bundled translation on one source.

## To promote a staged pack

1. Move `kjv.jsonl.gz`, `kjv.jsonl.gz.sha256` and `kjv.manifest.json` into
   `src-tauri/assets/translations/`.
2. Set `"enabled": true` in the manifest.
3. Update FR-59 in `docs/PRD.md` so the bundled list matches what ships.
4. Check the installer is still under 40 MB — CI enforces this, but a local
   `npm run tauri:build` is faster feedback.
