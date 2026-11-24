-- Visualiseringen teller og lister meldinger med signaturfeil og meldinger
-- uten trace_id. Delvise indekser holder dette raskt uten å skanne hele tabellen.
CREATE INDEX record_metadata_signaturfeil ON record_metadata (id)
    WHERE signature_status NOT IN ('valid', 'unsigned_before_required');
CREATE INDEX record_metadata_mangler_trace_id ON record_metadata (topic_id)
    WHERE trace_id IS NULL;
