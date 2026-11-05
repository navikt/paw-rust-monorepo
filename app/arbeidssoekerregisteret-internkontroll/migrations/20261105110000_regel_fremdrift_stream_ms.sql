ALTER TABLE regel_fremdrift ADD COLUMN siste_stream_ms BIGINT;

COMMENT ON COLUMN regel_fremdrift.siste_stream_ms IS
    'Høyeste stream-tid (ms) blant records regelen har sjekket. NULL før første record. Høyeste brukes fordi records fra ulike partisjoner kommer om hverandre.';
