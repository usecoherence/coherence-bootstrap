-- M1 spec-module schema
-- Logical owner: spec module (co-located in repo-local core-db Dolt database)
-- Scope: Spec, AcceptanceCriterion, ConcernKind repeats, and SpecRelation records.

CREATE TABLE IF NOT EXISTS specs (
  id VARCHAR(191) PRIMARY KEY,
  slug VARCHAR(191) NOT NULL,
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  level VARCHAR(32) NOT NULL,
  status VARCHAR(32) NOT NULL,
  created_at VARCHAR(64) NOT NULL,
  updated_at VARCHAR(64) NOT NULL
);

CREATE TABLE IF NOT EXISTS acceptance_criteria (
  id VARCHAR(191) PRIMARY KEY,
  spec_id VARCHAR(191) NOT NULL,
  title TEXT NOT NULL,
  intent TEXT NOT NULL,
  review_mode VARCHAR(32) NOT NULL,
  risk_level VARCHAR(32) NOT NULL,
  created_at VARCHAR(64) NOT NULL,
  updated_at VARCHAR(64) NOT NULL
);

CREATE TABLE IF NOT EXISTS acceptance_criterion_concerns (
  ac_id VARCHAR(191) NOT NULL,
  concern_kind VARCHAR(64) NOT NULL,
  PRIMARY KEY (ac_id, concern_kind)
);

CREATE TABLE IF NOT EXISTS spec_relations (
  id VARCHAR(191) PRIMARY KEY,
  source_spec_id VARCHAR(191) NOT NULL,
  target_spec_id VARCHAR(191) NOT NULL,
  relation_kind VARCHAR(64) NOT NULL,
  note TEXT NOT NULL,
  created_at VARCHAR(64) NOT NULL,
  updated_at VARCHAR(64) NOT NULL
);
