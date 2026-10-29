-- Visualiseringen teller og henter siste record uten trace_id per topic.
-- Bygges i migreringens transaksjon: blokkerer skriving til record_derived
-- mens den bygges, men rulles tilbake rent om podden stoppes underveis.
CREATE INDEX record_derived_uten_trace_id_idx
    ON record_derived (topic_id, xid)
    WHERE trace_id IS NULL;
