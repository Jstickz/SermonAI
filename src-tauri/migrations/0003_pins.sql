-- Pinned detections (FR-64, PRD §13.3, M3 deliverable 10).
--
-- A pastor returns to an anchor verse several times in a sermon. A pinned
-- card never auto-dismisses and can be sent live any number of times. The
-- pin is written here so the summary PDF (M4, §13.11) can mark anchor verses,
-- and so a service recovered after a restart (M4, FR-34) can restore them.
-- Within M3 the column is written, not read back.
ALTER TABLE detected_scriptures ADD COLUMN pinned BOOLEAN NOT NULL DEFAULT 0;
