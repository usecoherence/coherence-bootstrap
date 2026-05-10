-- module: spec
-- owner: spec
-- applied_by: coherence-core-db migration library (refinery)
-- id: spec/V2__acceptance_criteria_slug
-- depends_on: spec/V1__create_spec_tables

ALTER TABLE acceptance_criteria
  ADD COLUMN slug VARCHAR(191) NULL AFTER spec_id;

UPDATE acceptance_criteria
  SET slug = LOWER(REPLACE(id, '_', '-'))
  WHERE slug IS NULL OR slug = '';

ALTER TABLE acceptance_criteria
  MODIFY COLUMN slug VARCHAR(191) NOT NULL;

CREATE UNIQUE INDEX idx_acceptance_criteria_spec_id_slug
  ON acceptance_criteria (spec_id, slug);
