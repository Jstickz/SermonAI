-- Bible source moves from API.Bible to YouVersion Platform (PRD v2.2).
--
-- Two things change beyond the vendor. Tables are renamed to the names v2.2
-- uses, and every cached verse now carries the copyright attribution of the
-- version it came from: YouVersion's terms require attribution to be displayed
-- wherever the text is shown, so it has to travel with the text rather than be
-- looked up hopefully at render time.

ALTER TABLE bible_verses RENAME TO bible_cache;
ALTER TABLE translations RENAME TO bible_versions;

-- Attribution for the cached text.
--
-- Existing rows get an empty placeholder and a zero timestamp, which the app
-- reads as "never verified". Anything with attribution_updated_at = 0 must be
-- refreshed from YouVersion before its text is displayed again; an empty
-- attribution is never rendered as if it were valid.
ALTER TABLE bible_cache ADD COLUMN attribution TEXT NOT NULL DEFAULT '';
ALTER TABLE bible_cache ADD COLUMN attribution_updated_at INTEGER NOT NULL DEFAULT 0;

-- The original markup as YouVersion returned it, kept alongside the sanitized
-- plain text. The projector wants the plain form; the summary PDF renders the
-- richer one (PRD §13.11).
ALTER TABLE bible_cache ADD COLUMN html TEXT;

CREATE INDEX idx_bible_cache_stale ON bible_cache(attribution_updated_at);

-- Version catalog.
ALTER TABLE bible_versions ADD COLUMN yvp_version_id INTEGER;
ALTER TABLE bible_versions ADD COLUMN attribution TEXT NOT NULL DEFAULT '';
ALTER TABLE bible_versions ADD COLUMN license_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE bible_versions ADD COLUMN last_verified_at INTEGER NOT NULL DEFAULT 0;

-- api_bible_id is dead with the vendor change. SQLite 3.35+ supports dropping
-- a column, and rusqlite bundles a newer SQLite than that.
ALTER TABLE bible_versions DROP COLUMN api_bible_id;

-- Rows seeded under the old vendor name.
UPDATE bible_versions SET source = 'youversion' WHERE source = 'api_bible';

CREATE UNIQUE INDEX idx_bible_versions_yvp ON bible_versions(yvp_version_id)
  WHERE yvp_version_id IS NOT NULL;
