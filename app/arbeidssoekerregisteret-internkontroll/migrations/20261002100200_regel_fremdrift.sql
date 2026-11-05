-- Fremdrift per regel og topic-partisjon. En regel leser aldri på tvers av
-- partisjoner, bare på tvers av topics.
--
-- siste_stream_ms er stream_time etter siste_offset, regnet ut fra record_metadata:
--   record_timestamp_ms - LEAST(timestamp_delta_ms, 0)
-- Mangler record_timestamp_ms, beholdes forrige verdi.
CREATE TABLE regel_fremdrift (
    regel           SMALLINT NOT NULL,
    regelversjon    INTEGER  NOT NULL,
    topic_id        SMALLINT NOT NULL REFERENCES topic (id),
    topic_partition INTEGER  NOT NULL,
    siste_offset    BIGINT   NOT NULL,
    siste_stream_ms BIGINT,
    PRIMARY KEY (regel, regelversjon, topic_id, topic_partition)
);
