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

-- Simple operation type of a trace, mapped in code.
-- Used to determin what kind of operation this is.
CREATE TABLE trace_operation_type (
    trace_id        uuid NOT NULL,
    operation_type  SMALLINT NOT NULL,
    PRIMARY KEY (trace_id)
);

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

-- Signing keys known to the application, registered at startup from the embedded keys.
-- Never registered from record headers: the key ID in a header is an untrusted claim.
-- Never truncated on replay: record_derived refers to the IDs.
CREATE TABLE signing_key (
    id     SMALLINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    key_id VARCHAR(128) NOT NULL UNIQUE,
    -- Revoked key: the last stream time (ms) the key is accepted for.
    -- NULL when the key is not revoked. Set from code, never from record headers.
    revoked_after_ms   BIGINT,
    -- When the revocation was registered in code. Set from code with revoked_after_ms.
    -- Records derived before this time may need a new derivation.
    revocation_recorded_at TIMESTAMPTZ,
    CHECK ((revoked_after_ms IS NULL) = (revocation_recorded_at IS NULL))
);

CREATE TYPE signature_status AS ENUM
    ('valid', 'missing_signature', 'unknown_key', 'invalid_signature', 'technical_error');

-- Values derived from one Kafka record. One row per record, latest derivation only.
CREATE TABLE record_derived (
    topic_id           SMALLINT NOT NULL,
    record_partition   INTEGER NOT NULL,
    record_offset      BIGINT NOT NULL,
    sequence_number    BIGINT GENERATED ALWAYS AS IDENTITY,
    signature_status   signature_status NOT NULL,
    -- Required for 'valid' and 'invalid_signature', where the key was known.
    -- Optional for 'technical_error': set when the error came after the key lookup.
    -- Never set for 'missing_signature' and 'unknown_key'.
    signing_key_id     SMALLINT REFERENCES signing_key (id),
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
    CHECK (CASE signature_status
        WHEN 'valid' THEN signing_key_id IS NOT NULL
        WHEN 'invalid_signature' THEN signing_key_id IS NOT NULL
        WHEN 'technical_error' THEN TRUE
        ELSE signing_key_id IS NULL
    END)
);

CREATE INDEX record_derived_trace_id_idx
    ON record_derived (trace_id)
    WHERE trace_id IS NOT NULL;
