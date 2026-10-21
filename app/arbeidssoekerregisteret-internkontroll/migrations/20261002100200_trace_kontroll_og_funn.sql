CREATE TABLE regel_fremdrift (
    regel                  SMALLINT NOT NULL,
    regelversjon           INTEGER NOT NULL,
    siste_sequence_number  BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (regel, regelversjon)
);

CREATE TABLE funn (
    trace_id        UUID NOT NULL REFERENCES trace (trace_id),
    regel           SMALLINT NOT NULL,
    regelversjon    INTEGER NOT NULL,
    utfall          SMALLINT NOT NULL,
    opprettet       TIMESTAMPTZ NOT NULL,
    sist_oppdatert  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (trace_id, regel, regelversjon)
);

CREATE INDEX funn_avvik_idx ON funn (regel, regelversjon, opprettet) WHERE utfall <> 0;
