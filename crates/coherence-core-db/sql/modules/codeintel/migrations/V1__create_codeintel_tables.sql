-- module: codeintel
-- owner: codeintel module
-- applied_by: coherence-core-db migration library (refinery); tracked in refinery_schema_history_codeintel
-- id: codeintel/V1__create_codeintel_tables
-- depends_on: project DB initialized; spec module schema (acceptance_criteria IDs) may exist but is not enforced here
--
-- M1: No foreign keys from ac_id -> acceptance_criteria or code_location_id -> codeintel_code_locations.
-- Links are logical only until a later milestone adds referential integrity if desired.

CREATE TABLE IF NOT EXISTS codeintel_code_locations (
  id VARCHAR(191) PRIMARY KEY,
  repo_path TEXT NOT NULL,
  file_path TEXT NOT NULL,
  kind VARCHAR(64) NOT NULL,
  symbol TEXT,
  test_command TEXT,
  created_at VARCHAR(64) NOT NULL,
  updated_at VARCHAR(64) NOT NULL
);

-- relation_kind: verified_by | implemented_by | touched_by (application-enforced; column is VARCHAR for evolution)
CREATE TABLE IF NOT EXISTS codeintel_ac_links (
  id VARCHAR(191) PRIMARY KEY,
  ac_id VARCHAR(191) NOT NULL,
  code_location_id VARCHAR(191) NOT NULL,
  relation_kind VARCHAR(64) NOT NULL,
  note TEXT NOT NULL,
  created_at VARCHAR(64) NOT NULL,
  updated_at VARCHAR(64) NOT NULL
);
