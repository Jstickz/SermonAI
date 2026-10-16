# ADR 0006: On-demand packs and a hard installer size gate

**Decided:** 7 September 2026 · **Status:** Accepted · **Full record:** `docs/adr/0006-on-demand-packs.md`

The base installer carries only what is needed to project a verse: the app, KJV/WEB/ASV, the int8 verse index and encoder, the default theme. Everything else is a pack: ranged resumable download, SHA-256 verified before activation, clear size label. **CI fails the build if any installer exceeds 40 MB.** That is a gate, not a guideline: it is the mechanism that stops heavy assets quietly landing in the base download.

Every future feature answers "base or pack?" before it is built. This is why the encoder upgrade prompt (`docs/SermonAI_Qwen3_Embedding_Prompt.md`) says anything that breaks the gate ships as an optional pack, and why NDI is a pack.

Measured today: msi 5.12 MB, translations 3.86 MB, index 7.71 MB, encoder 7.97 MB, projected ~25 MB with ~15 MB spare.

Still open: the pack system has only ever run against a local range-capable server, never a real CDN bucket; manifest signatures are not verified; archive extraction is unbuilt. See [[open-questions]]. Related: [[adr-0001-tauri-over-electron]], [[sessions/2026-09-07]]
