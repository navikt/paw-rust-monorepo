-- Marker per rule: xid of the last record_derived row the rule has checked.
-- record_derived.xid is unique, so xid alone identifies the row.
-- '0' is never assigned to a real transaction, so the start marker is below every row.
CREATE TABLE regel_fremdrift (
    regel         SMALLINT NOT NULL,
    regelversjon  INTEGER NOT NULL,
    siste_xid     xid8 NOT NULL DEFAULT '0',
    PRIMARY KEY (regel, regelversjon)
);

-- Current state of a trace for a rule. A rule that finds the trace OK deletes
-- the row, so OK is never stored. opprettet is kept on upsert and tells how
-- long the trace has been failing.
CREATE TABLE funn (
    trace_id        UUID NOT NULL REFERENCES trace (trace_id),
    regel           SMALLINT NOT NULL,
    regelversjon    INTEGER NOT NULL,
    utfall          SMALLINT NOT NULL,
    opprettet       TIMESTAMPTZ NOT NULL,
    sist_oppdatert  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (trace_id, regel, regelversjon)
);

CREATE INDEX funn_opprettet_idx ON funn (regel, regelversjon, opprettet);
