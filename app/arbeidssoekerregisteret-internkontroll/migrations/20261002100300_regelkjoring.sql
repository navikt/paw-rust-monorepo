-- Lås for regelkjøring, én rad per partisjonsnummer på tvers av topics.
-- Radene brukes bare som lås: regelkjøringen holder dem med
-- FOR UPDATE SKIP LOCKED gjennom hele runden.
CREATE TABLE regel_partisjon_laas (
    topic_partition INTEGER PRIMARY KEY
);

CREATE TYPE alvorlighet AS ENUM ('info', 'advarsel', 'kritisk');

-- Gjeldende funn per regelversjon og subjekt. record_metadata_id peker på
-- meldingen som sist bestemte funnet; nivået til regelen sier hva subjektet er.
CREATE TABLE funn (
    regel              SMALLINT    NOT NULL,
    regelversjon       INTEGER     NOT NULL,
    record_metadata_id BIGINT      NOT NULL REFERENCES record_metadata (id) ON DELETE CASCADE,
    kode               SMALLINT    NOT NULL,
    alvorlighet        alvorlighet NOT NULL,
    PRIMARY KEY (regel, regelversjon, record_metadata_id)
);

-- Funn der regelen har kommet minst ett minutt forbi meldingen i stream time.
-- Yngre funn kan fortsatt bli fjernet av en melding som ikke er lest ennå.
CREATE VIEW modne_funn AS
SELECT f.*
FROM funn f
JOIN record_metadata rm ON rm.id = f.record_metadata_id
WHERE rm.record_timestamp_ms IS NULL
   OR rm.record_timestamp_ms + 60000 <= (
       SELECT MAX(rf.siste_stream_ms) FROM regel_fremdrift rf
       WHERE rf.regel = f.regel AND rf.regelversjon = f.regelversjon
         AND rf.topic_partition = rm.record_partition);
