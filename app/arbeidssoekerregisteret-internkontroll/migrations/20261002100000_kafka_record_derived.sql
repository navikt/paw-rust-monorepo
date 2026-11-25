CREATE TABLE signing_key (
    id     SMALLINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    key_id VARCHAR(128) NOT NULL UNIQUE CHECK (key_id <> '')
);

CREATE TABLE topic (
    id   SMALLINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name VARCHAR(249) NOT NULL UNIQUE
);

CREATE TABLE stream_time (
    topic_id            SMALLINT NOT NULL REFERENCES topic (id),
    topic_partition     INTEGER NOT NULL,

    timestamp_ms        BIGINT NOT NULL,
    PRIMARY KEY (topic_id, topic_partition)
);


-- Simple type reflecting kafka timestamp types.
CREATE TYPE kafka_timestamp_type AS ENUM ('not_available', 'create_time', 'log_append_time');

CREATE TYPE signature_status AS ENUM
    ('valid', 'signature_mismatch', 'malformed_signature', 'key_revoked', 'payload_too_large',
     'missing_signature', 'unsigned_before_required', 'unknown_key', 'no_record_timestamp', 'technical_error');

-- Values derived from one Kafka record. One row per record, latest derivation only.
CREATE TYPE payload_status AS ENUM ('ok', 'ugyldig');

CREATE TYPE meldingstype AS ENUM (
    'hendelse_startet', 'hendelse_avsluttet', 'hendelse_avvist',
    'hendelse_avvist_stopp_av_periode', 'hendelse_arbeidssoeker_id_flettet_inn',
    'hendelse_automatisk_id_merge_ikke_mulig', 'hendelse_identitetsnummer_sammenslaatt',
    'hendelse_opplysninger_om_arbeidssoeker_mottatt',
    'bekreftelse_hendelse_leveringsfrist_utloept',
    'bekreftelse_hendelse_ekstern_grace_periode_utloept',
    'bekreftelse_hendelse_register_grace_periode_utloept', 'bekreftelse_hendelse_tilgjengelig',
    'bekreftelse_hendelse_melding_mottatt', 'bekreftelse_hendelse_periode_avsluttet',
    'bekreftelse_hendelse_register_grace_periode_gjenstaaende_tid',
    'bekreftelse_hendelse_ba_om_aa_avslutte_periode',
    'bekreftelse_hendelse_paa_vegne_av_startet',
    'bekreftelse_hendelse_grace_utloept_etter_ekstern_innsamling',
    'periode_startet', 'periode_avsluttet', 'opplysninger', 'profilering', 'egenvurdering',
    'bekreftelse', 'paa_vegne_av_start', 'paa_vegne_av_stopp'
);

CREATE TYPE kilde AS ENUM (
    'inngang-api',
    'bekreftelse',
    'bekreftelse-tjeneste',
    'dolly',
    'ukjent',
    'ikke_aktuelt'
);

CREATE TYPE System AS ENUM (
    'arbeidssoekerregisteret',
    'dagpenger',
    'friskmeldt_til_arbeidsformidling'
);

CREATE TYPE utfoert_av_type AS ENUM ('udefinert', 'ukjent_verdi', 'system', 'sluttbruker', 'veileder');

CREATE TABLE record_metadata (
    id                 BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    topic_id           SMALLINT NOT NULL,
    record_partition   INTEGER NOT NULL,
    record_offset      BIGINT NOT NULL,
    record_timestamp_type kafka_timestamp_type NOT NULL,
    record_timestamp_ms   BIGINT,
    signature_status   signature_status NOT NULL,
    signing_key_id     BIGINT,
    -- record_timestamp_ms minus the highest timestamp of earlier offsets
    -- in the same topic-partition. NULL when there is nothing to compare with.
    timestamp_delta_ms BIGINT,
    -- From the last traceparent header. NULL when missing or invalid,
    -- including the all-zero trace ID.
    trace_id           UUID,
    kilde              kilde,
    system             System,
    payload_status     payload_status NOT NULL,
    meldingstype       meldingstype,
    periode_id         UUID,
    arbeidssoeker_id   BIGINT,
    identitetsnummer   TEXT,
    utfoert_tidspunkt_ms BIGINT,
    utfoert_av_type    utfoert_av_type,
    utfoert_av_id      TEXT,
    derived_at         TIMESTAMPTZ NOT NULL,
    UNIQUE (topic_id, record_partition, record_offset),
    FOREIGN KEY (signing_key_id) REFERENCES signing_key (id),
    CHECK ((signing_key_id IS NOT NULL) = (signature_status IN
        ('valid', 'signature_mismatch', 'malformed_signature', 'key_revoked', 'payload_too_large'))),
    CHECK ((utfoert_tidspunkt_ms IS NULL) = (utfoert_av_type IS NULL)
       AND (utfoert_av_type IS NULL) = (utfoert_av_id IS NULL))
);

-- Mirror of one Kafka record. Headers are stored as two parallel arrays:
-- element i of header_keys and header_values is header number i, in Kafka order.
CREATE TABLE kafka_record (
    metadata_id           BIGINT PRIMARY KEY REFERENCES record_metadata (id) ON DELETE CASCADE,
    record_key            BYTEA,
    record_value          BYTEA,
    header_keys           TEXT[] NOT NULL,
    header_values         BYTEA[] NOT NULL
);

