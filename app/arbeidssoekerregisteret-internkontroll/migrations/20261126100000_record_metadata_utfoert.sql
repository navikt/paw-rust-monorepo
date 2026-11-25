CREATE TYPE utfoert_av_type AS ENUM ('udefinert', 'ukjent_verdi', 'system', 'sluttbruker', 'veileder');

ALTER TABLE record_metadata
    ADD COLUMN identitetsnummer     TEXT,
    ADD COLUMN utfoert_tidspunkt_ms BIGINT,
    ADD COLUMN utfoert_av_type      utfoert_av_type,
    ADD COLUMN utfoert_av_id        TEXT,
    ADD CHECK ((utfoert_tidspunkt_ms IS NULL) = (utfoert_av_type IS NULL)
           AND (utfoert_av_type IS NULL) = (utfoert_av_id IS NULL));
