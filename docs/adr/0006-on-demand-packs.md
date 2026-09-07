# ADR 0006 — On-demand packs and a hard installer size gate

- **Status:** Accepted
- **Date:** 7 September 2026
- **PRD:** §8.11 (FR-59 to FR-63), §9.7, §14.3, §15.5

## Context

The app must install in under a minute on a church laptop, without admin rights,
often over a slow connection — against a competitor with a 2 GB footprint. But
the features churches want most offline (whisper.cpp models, twenty-plus
translations, an optional local LLM) are individually larger than the entire
target installer.

## Decision

Split the download. The base installer carries only what is needed to project a
verse: the app, KJV/WEB/ASV, the quantized verse index, and the default theme.
Everything else is a **pack**, downloaded from inside the app with a clear size
label, a progress bar, pause/resume, and a checksum.

- Packs are fetched with ranged requests, resumable across app restarts and
  network drops, and verified by SHA-256 before activation. A partial pack is
  never activated (FR-61).
- The manifest is signed and served from the Pack CDN.
- Updates ship as delta patches through the Tauri updater, so a routine release
  is a few MB and never re-downloads a pack (FR-62).
- **CI fails the build if either installer exceeds 40 MB.** This is a gate, not
  a guideline: it is the mechanism that stops new heavy assets from quietly
  landing in the base download.

## Consequences

**Good**
- "About sixty seconds to Sunday" is a promise the product can keep.
- A church pays disk and bandwidth only for what it actually uses.
- NDI, which carries its own SDK licensing and binary weight, becomes an
  optional pack rather than a tax on every install.

**Costs**
- A resumable, checksummed, pausable downloader is real work in M0, before any
  user-visible feature exists.
- First-run offline is limited: a church with no internet at install time gets
  three translations and no offline speech until it can download once. The
  onboarding wizard has to be honest about this.
- Pack hosting and CDN invalidation become operational responsibilities.
- Every future feature must answer "base or pack?" before it is built.
