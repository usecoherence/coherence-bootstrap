CREATE TABLE IF NOT EXISTS coherence_task_ac_link (
  id TEXT PRIMARY KEY,
  beads_task_id TEXT NOT NULL,
  ac_id TEXT NOT NULL,
  link_kind TEXT NOT NULL,
  reason TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS coherence_task_attempt (
  id TEXT PRIMARY KEY,
  beads_task_id TEXT NOT NULL,
  attempt_no INTEGER NOT NULL,
  status TEXT NOT NULL,
  reason TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL
);
