-- Marker per rule: xid of the last record_derived row the rule has checked.
-- record_derived.xid is unique, so xid alone identifies the row.
-- '0' is never assigned to a real transaction, so the start marker is below every row.
CREATE TABLE regel_fremdrift (
    regel         SMALLINT NOT NULL,
    regelversjon  INTEGER NOT NULL,
    siste_xid     xid8 NOT NULL DEFAULT '0',
    PRIMARY KEY (regel, regelversjon)
);

-- Rekkefølgen bestemmer sorteringen, med kritiske funn først ved DESC.
CREATE TYPE alvorlighet AS ENUM ('info', 'advarsel', 'kritisk');

-- Alvorlighet er denormalisert fra (regel, utfall). Endring av alvorlighet for
-- et utfall krever at regelversjonen økes.
CREATE TABLE funn_melding (
    xid                       xid8 NOT NULL,
    regel                     SMALLINT NOT NULL,
    regelversjon              INTEGER NOT NULL,
    utfall                    SMALLINT NOT NULL,
    alvorlighet               alvorlighet NOT NULL,
    opprettet                 TIMESTAMPTZ NOT NULL,
    sist_oppdatert            TIMESTAMPTZ NOT NULL,
    opprettet_stream_ms       BIGINT NOT NULL,
    sist_oppdatert_stream_ms  BIGINT NOT NULL,
    PRIMARY KEY (xid, regel, regelversjon)
);

COMMENT ON COLUMN funn_melding.alvorlighet IS
    'Denormalisert fra (regel, utfall). Endring av alvorlighet for et utfall krever bump av regelversjon.';
CREATE INDEX funn_melding_visning_idx
    ON funn_melding (alvorlighet, regel, opprettet_stream_ms DESC);
COMMENT ON INDEX funn_melding_visning_idx IS
    'Treffer fanespørringen med filter på alvorlighet og regel, nyeste først.';

CREATE TABLE funn_trace (
    trace_id                  UUID NOT NULL REFERENCES trace (trace_id),
    regel                     SMALLINT NOT NULL,
    regelversjon              INTEGER NOT NULL,
    utfall                    SMALLINT NOT NULL,
    alvorlighet               alvorlighet NOT NULL,
    opprettet                 TIMESTAMPTZ NOT NULL,
    sist_oppdatert            TIMESTAMPTZ NOT NULL,
    opprettet_stream_ms       BIGINT NOT NULL,
    sist_oppdatert_stream_ms  BIGINT NOT NULL,
    PRIMARY KEY (trace_id, regel, regelversjon)
);

COMMENT ON COLUMN funn_trace.alvorlighet IS
    'Denormalisert fra (regel, utfall). Endring av alvorlighet for et utfall krever bump av regelversjon.';
CREATE INDEX funn_trace_visning_idx
    ON funn_trace (alvorlighet, regel, opprettet_stream_ms DESC);
COMMENT ON INDEX funn_trace_visning_idx IS
    'Treffer fanespørringen med filter på alvorlighet og regel, nyeste først.';

CREATE TABLE funn_periode (
    periode_id                UUID NOT NULL,
    regel                     SMALLINT NOT NULL,
    regelversjon              INTEGER NOT NULL,
    utfall                    SMALLINT NOT NULL,
    alvorlighet               alvorlighet NOT NULL,
    opprettet                 TIMESTAMPTZ NOT NULL,
    sist_oppdatert            TIMESTAMPTZ NOT NULL,
    opprettet_stream_ms       BIGINT NOT NULL,
    sist_oppdatert_stream_ms  BIGINT NOT NULL,
    PRIMARY KEY (periode_id, regel, regelversjon)
);

COMMENT ON COLUMN funn_periode.alvorlighet IS
    'Denormalisert fra (regel, utfall). Endring av alvorlighet for et utfall krever bump av regelversjon.';
CREATE INDEX funn_periode_visning_idx
    ON funn_periode (alvorlighet, regel, opprettet_stream_ms DESC);
COMMENT ON INDEX funn_periode_visning_idx IS
    'Treffer fanespørringen med filter på alvorlighet og regel, nyeste først.';

CREATE TABLE funn_arbeidssoeker (
    arbeidssoeker_id          BIGINT NOT NULL,
    regel                     SMALLINT NOT NULL,
    regelversjon              INTEGER NOT NULL,
    utfall                    SMALLINT NOT NULL,
    alvorlighet               alvorlighet NOT NULL,
    opprettet                 TIMESTAMPTZ NOT NULL,
    sist_oppdatert            TIMESTAMPTZ NOT NULL,
    opprettet_stream_ms       BIGINT NOT NULL,
    sist_oppdatert_stream_ms  BIGINT NOT NULL,
    PRIMARY KEY (arbeidssoeker_id, regel, regelversjon)
);

COMMENT ON COLUMN funn_arbeidssoeker.alvorlighet IS
    'Denormalisert fra (regel, utfall). Endring av alvorlighet for et utfall krever bump av regelversjon.';
CREATE INDEX funn_arbeidssoeker_visning_idx
    ON funn_arbeidssoeker (alvorlighet, regel, opprettet_stream_ms DESC);
COMMENT ON INDEX funn_arbeidssoeker_visning_idx IS
    'Treffer fanespørringen med filter på alvorlighet og regel, nyeste først.';
