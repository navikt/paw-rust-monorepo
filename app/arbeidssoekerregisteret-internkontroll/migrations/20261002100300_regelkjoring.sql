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
