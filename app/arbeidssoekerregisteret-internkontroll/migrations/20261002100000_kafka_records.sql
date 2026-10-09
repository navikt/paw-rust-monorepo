CREATE TABLE kafka_record_data (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    record_key   BYTEA,
    record_value BYTEA
);

CREATE TABLE kafka_header (
    data_id  BIGINT NOT NULL REFERENCES kafka_record_data (id),
    position INTEGER NOT NULL,
    key      TEXT NOT NULL,
    value    BYTEA,
    PRIMARY KEY (data_id, position)
);

CREATE TABLE kafka_record (
    id                       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    topic                    VARCHAR(255) NOT NULL,
    partition                INTEGER NOT NULL,
    offset                   BIGINT NOT NULL,
    timestamp_ms             BIGINT NOT NULL,
    observed_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    data_id                  BIGINT NOT NULL UNIQUE REFERENCES kafka_record_data (id),
    traceparent_parse_status VARCHAR(32) NOT NULL,
    trace_id                 VARCHAR(32),
    parent_id                VARCHAR(16),
    trace_version            VARCHAR(2),
    trace_flags              VARCHAR(2),
    signature_status         VARCHAR(32) NOT NULL,
    signing_key_id           VARCHAR(128),
    UNIQUE (topic, partition, offset)
);

CREATE INDEX kafka_record_trace_id_idx
    ON kafka_record (trace_id)
    WHERE trace_id IS NOT NULL;

