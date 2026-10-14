CREATE TYPE trace_kontroll_status AS ENUM ('open', 'done');

CREATE TABLE trace_kontroll (
    id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    trace_id         UUID NOT NULL,
    -- Rule ID, mapped in code. Never reuse or renumber an ID.
    regel            SMALLINT NOT NULL,
    regelversjon     INTEGER NOT NULL,
    status           trace_kontroll_status NOT NULL,
    neste_vurdering  TIMESTAMPTZ NOT NULL,
    sist_vurdert     TIMESTAMPTZ,
    UNIQUE (trace_id, regel, regelversjon)
);

CREATE INDEX trace_kontroll_arbeidsliste_idx
    ON trace_kontroll (status, neste_vurdering);

CREATE TYPE funn_status AS ENUM ('ok', 'failed', 'technical_error');

-- Result of one trace check. Findings always belong to a trace, never to a single record.
CREATE TABLE funn (
    id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- Finding type ID, mapped in code. Never reuse or renumber an ID.
    type              SMALLINT NOT NULL,
    status            funn_status NOT NULL,
    opprettet         TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Optional: the record in the trace the finding points to.
    topic_id          SMALLINT,
    record_partition  INTEGER,
    record_offset     BIGINT,
    trace_kontroll_id BIGINT NOT NULL UNIQUE REFERENCES trace_kontroll (id),
    FOREIGN KEY (topic_id, record_partition, record_offset)
        REFERENCES kafka_record (topic_id, record_partition, record_offset)
);

CREATE INDEX funn_kafka_record_idx
    ON funn (topic_id, record_partition, record_offset)
    WHERE topic_id IS NOT NULL;
