CREATE TABLE IF NOT EXISTS coherence_tool_capability (
  id TEXT PRIMARY KEY,
  command TEXT NOT NULL,
  status TEXT NOT NULL,
  reason TEXT NOT NULL DEFAULT ''
);
