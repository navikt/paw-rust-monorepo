-- hent_grunnlag filtrerer på nøkkel og xid <= $2 og sorterer på xid.
-- Med xid i indeksen slipper Postgres å sortere grunnlaget for hver record.
-- Bygges i migreringens transaksjon: blokkerer skriving til record_derived
-- mens den bygges, men rulles tilbake rent om podden stoppes underveis.
DROP INDEX record_derived_trace_id_idx;
CREATE INDEX record_derived_trace_id_idx
    ON record_derived (trace_id, xid)
    WHERE trace_id IS NOT NULL;

DROP INDEX record_derived_periode_id_idx;
CREATE INDEX record_derived_periode_id_idx
    ON record_derived (periode_id, xid)
    WHERE periode_id IS NOT NULL;

DROP INDEX record_derived_arbeidssoeker_id_idx;
CREATE INDEX record_derived_arbeidssoeker_id_idx
    ON record_derived (arbeidssoeker_id, xid)
    WHERE arbeidssoeker_id IS NOT NULL;
