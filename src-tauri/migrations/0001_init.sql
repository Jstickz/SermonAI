-- SermonAI initial schema. Transcribed from PRD §14.2; change the PRD first.

CREATE TABLE bible_verses (
  id INTEGER PRIMARY KEY,
  translation_code TEXT NOT NULL,
  book_id TEXT NOT NULL,
  chapter INTEGER NOT NULL,
  verse INTEGER NOT NULL,
  text TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(translation_code, book_id, chapter, verse)
);
CREATE INDEX idx_verse_lookup ON bible_verses(translation_code, book_id, chapter, verse);

CREATE TABLE translations (
  code TEXT PRIMARY KEY,
  full_name TEXT NOT NULL,
  language TEXT NOT NULL,
  copyright_status TEXT,
  source TEXT NOT NULL,              -- 'api_bible' | 'import'
  api_bible_id TEXT,
  imported_from TEXT,
  downloaded_at TIMESTAMP
);

CREATE TABLE series (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  description TEXT,
  book_template TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sermons (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT,
  preacher TEXT,
  date TIMESTAMP NOT NULL,
  duration_seconds INTEGER,
  default_translation TEXT,
  series_id INTEGER REFERENCES series(id),
  status TEXT NOT NULL DEFAULT 'live', -- live | processing | ready | needs_attention
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE transcript_segments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sermon_id INTEGER NOT NULL REFERENCES sermons(id),
  start_time_ms INTEGER NOT NULL,
  end_time_ms INTEGER NOT NULL,
  text TEXT NOT NULL,
  confidence REAL
);
CREATE INDEX idx_transcript_sermon ON transcript_segments(sermon_id, start_time_ms);

CREATE TABLE detected_scriptures (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sermon_id INTEGER NOT NULL REFERENCES sermons(id),
  segment_id INTEGER REFERENCES transcript_segments(id),
  book TEXT NOT NULL,
  chapter INTEGER NOT NULL,
  verse INTEGER NOT NULL,
  end_verse INTEGER,
  translation TEXT NOT NULL,
  confidence REAL,
  source TEXT,                        -- regex | vector | llm | manual
  accepted_by_operator BOOLEAN,
  went_live BOOLEAN,
  displayed_at TIMESTAMP
);

CREATE TABLE sermon_summaries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sermon_id INTEGER NOT NULL REFERENCES sermons(id),
  version INTEGER NOT NULL DEFAULT 1,
  generator TEXT NOT NULL,            -- claude | local | template
  template TEXT NOT NULL,             -- sermon_brief | study_guide | devotional | minimal
  summary_json TEXT NOT NULL,         -- structured sections (see PRD §26.3)
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(sermon_id, version)
);

CREATE TABLE exports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sermon_id INTEGER REFERENCES sermons(id),
  series_id INTEGER REFERENCES series(id),
  summary_id INTEGER REFERENCES sermon_summaries(id),
  kind TEXT NOT NULL,                 -- summary_pdf | transcript_pdf | transcript_docx | book_pdf | book_docx | slides_pptx | derivative
  file_path TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE remote_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  device_label TEXT,
  token_hash TEXT NOT NULL,
  paired_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  revoked_at TIMESTAMP
);

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Full-text search over the archive (FR-56).
CREATE VIRTUAL TABLE transcript_fts USING fts5(text, content='transcript_segments', content_rowid='id');
CREATE VIRTUAL TABLE summary_fts USING fts5(summary_text, content='sermon_summaries', content_rowid='id');
