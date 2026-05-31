-- module: codeintel
-- owner: codeintel module
-- applied_by: coherence-core-db migration library (refinery); tracked in refinery_schema_history_codeintel
-- id: codeintel/V2__verification_latest
-- depends_on: codeintel/V1__create_codeintel_tables
--
-- Stores the latest verification result per AC and per verified link. Historical run evidence remains
-- outside canonical rows under `.coherence/runs/...` (ADR-0005); this table is intentionally a compact
-- latest-status cache for TUI and CLI reporting.

CREATE TABLE IF NOT EXISTS codeintel_ac_verification_latest (
  ac_id VARCHAR(191) PRIMARY KEY,
  overall_status VARCHAR(64) NOT NULL,
  no_verification_links BOOLEAN NOT NULL,
  link_count BIGINT NOT NULL,
  verified_at VARCHAR(64) NOT NULL
);

CREATE TABLE IF NOT EXISTS codeintel_ac_verification_link_latest (
  ac_id VARCHAR(191) NOT NULL,
  code_location_id VARCHAR(191) NOT NULL,
  command TEXT NOT NULL,
  status VARCHAR(64) NOT NULL,
  exit_code INT,
  skip_reason TEXT,
  output_summary TEXT NOT NULL,
  verified_at VARCHAR(64) NOT NULL,
  PRIMARY KEY (ac_id, code_location_id)
);
