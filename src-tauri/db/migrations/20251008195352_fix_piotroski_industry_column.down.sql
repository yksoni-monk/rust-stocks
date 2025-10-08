-- Revert: Drop the fixed Piotroski views
-- Note: Reverting will break Piotroski screening since the original views reference non-existent s.industry column

DROP VIEW IF EXISTS piotroski_screening_results;
DROP VIEW IF EXISTS piotroski_multi_year_data;
