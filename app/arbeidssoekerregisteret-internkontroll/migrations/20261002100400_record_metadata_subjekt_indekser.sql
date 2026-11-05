-- Regler slår opp meldinger per subjekt, og lagring av funn finner
-- tidligere funn for samme subjekt.
CREATE INDEX record_metadata_trace_id ON record_metadata (trace_id) WHERE trace_id IS NOT NULL;
CREATE INDEX record_metadata_periode_id ON record_metadata (periode_id) WHERE periode_id IS NOT NULL;
CREATE INDEX record_metadata_arbeidssoeker_id ON record_metadata (arbeidssoeker_id) WHERE arbeidssoeker_id IS NOT NULL;
