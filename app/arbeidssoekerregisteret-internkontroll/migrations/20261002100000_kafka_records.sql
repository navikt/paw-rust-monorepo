-- Physical Kafka topics, registered by the application at startup.
-- Never truncated on replay: records and findings refer to the IDs.
CREATE TABLE topic (
    id   SMALLINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name VARCHAR(249) NOT NULL UNIQUE
);

-- Highest timestamp seen in each topic-partition,
-- for computing timestamp deltas.
-- A record can be prevented  from incrementing the timestamp if it is
-- to far ahead of now (detrmined by app config).
CREATE TABLE stream_time (
    topic_id            SMALLINT NOT NULL REFERENCES topic (id),
    topic_partition     INTEGER NOT NULL,
    timestamp_ms        BIGINT NOT NULL,
    PRIMARY KEY (topic_id, topic_partition)
);

-- One row per trace ID. Reference times are stream times in ms.
-- operation_type is mapped in code and written once.
CREATE TABLE trace (
    sequence_number     BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    trace_id            UUID PRIMARY KEY,
    first_reference_ms  BIGINT NOT NULL,
    last_reference_ms   BIGINT NOT NULL,
    last_sequence_number BIGINT NOT NULL,
    operation_type      SMALLINT,
    CHECK (first_reference_ms <= last_reference_ms)
);

CREATE INDEX trace_first_reference_idx ON trace (first_reference_ms);
CREATE INDEX trace_last_sequence_idx ON trace (last_sequence_number);

-- Simple type reflecting kafka timestamp types.
CREATE TYPE kafka_timestamp_type AS ENUM ('not_available', 'create_time', 'log_append_time');

-- Mirror of one Kafka record. Headers are stored as two parallel arrays:
-- element i of header_keys and header_values is header number i, in Kafka order.
CREATE TABLE kafka_record (
    topic_id              SMALLINT NOT NULL REFERENCES topic (id),
    record_partition      INTEGER NOT NULL,
    record_offset         BIGINT NOT NULL,
    record_timestamp_type kafka_timestamp_type NOT NULL,
    record_timestamp_ms   BIGINT,
    record_key            BYTEA,
    record_value          BYTEA,
    header_keys           TEXT[] NOT NULL,
    header_values         BYTEA[] NOT NULL,
    PRIMARY KEY (topic_id, record_partition, record_offset)
);

CREATE TYPE signature_status AS ENUM
    ('valid', 'signature_mismatch', 'malformed_signature', 'key_revoked', 'payload_too_large',
     'missing_signature', 'unsigned_before_required', 'unknown_key', 'no_record_timestamp', 'technical_error');

-- Values derived from one Kafka record. One row per record, latest derivation only.
CREATE TABLE record_derived (
    topic_id           SMALLINT NOT NULL,
    record_partition   INTEGER NOT NULL,
    record_offset      BIGINT NOT NULL,
    sequence_number    BIGINT GENERATED ALWAYS AS IDENTITY,
    signature_status   signature_status NOT NULL,
    signing_key_id     VARCHAR(128) CHECK (signing_key_id <> ''),
    -- record_timestamp_ms minus the highest timestamp of earlier offsets
    -- in the same topic-partition. NULL when there is nothing to compare with.
    timestamp_delta_ms BIGINT,
    -- From the last traceparent header. NULL when missing or invalid,
    -- including the all-zero trace ID.
    trace_id           UUID,
    derived_at         TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (topic_id, record_partition, record_offset),
    FOREIGN KEY (topic_id, record_partition, record_offset)
        REFERENCES kafka_record (topic_id, record_partition, record_offset)
        ON DELETE CASCADE,
    CHECK ((signing_key_id IS NOT NULL) = (signature_status IN
        ('valid', 'signature_mismatch', 'malformed_signature', 'key_revoked', 'payload_too_large')))
);

CREATE INDEX record_derived_trace_id_idx
    ON record_derived (trace_id)
    WHERE trace_id IS NOT NULL;
