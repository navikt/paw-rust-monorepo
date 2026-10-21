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

CREATE TABLE arbeidssoeker (
    arbeidssoeker_id    BIGINT PRIMARY KEY,
    registrert          BOOLEAN NOT NULL,
    endret_av_trace_id  UUID REFERENCES trace (trace_id)
);

CREATE TABLE identitetsnummer (
    identitetsnummer  TEXT PRIMARY KEY,
    arbeidssoeker_id  BIGINT NOT NULL REFERENCES arbeidssoeker (arbeidssoeker_id)
);

CREATE INDEX identitetsnummer_arbeidssoeker_idx ON identitetsnummer (arbeidssoeker_id);

CREATE TABLE status_fremdrift (
    id                     BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    siste_sequence_number  BIGINT NOT NULL DEFAULT 0
);
