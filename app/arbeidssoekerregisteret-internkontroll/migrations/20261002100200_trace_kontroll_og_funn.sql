CREATE TABLE trace_kontroll (
    id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    trace_id         VARCHAR(32) NOT NULL,
    regel            VARCHAR(100) NOT NULL,
    regelversjon     INTEGER NOT NULL,
    status           VARCHAR(32) NOT NULL,
    neste_vurdering  TIMESTAMPTZ NOT NULL,
    sist_vurdert     TIMESTAMPTZ,
    UNIQUE (trace_id, regel, regelversjon)
);

CREATE INDEX trace_kontroll_arbeidsliste_idx
    ON trace_kontroll (status, neste_vurdering);

CREATE TABLE funn (
    id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    type              VARCHAR(100) NOT NULL,
    status            VARCHAR(32) NOT NULL,
    opprettet         TIMESTAMPTZ NOT NULL DEFAULT now(),
    kafka_record_id   BIGINT REFERENCES kafka_record (id),
    trace_kontroll_id BIGINT NOT NULL UNIQUE REFERENCES trace_kontroll (id)
);

CREATE INDEX funn_kafka_record_idx ON funn (kafka_record_id);
