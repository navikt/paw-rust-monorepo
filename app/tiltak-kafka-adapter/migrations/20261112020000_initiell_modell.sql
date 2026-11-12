CREATE TABLE arbeidssoekere
(
    periode_id       UUID PRIMARY KEY,
    identitetsnummer VARCHAR(20)  NOT NULL,
    periode_startet  TIMESTAMP(6) NOT NULL
);

CREATE TABLE tiltak_synk
(
    periode_id    UUID PRIMARY KEY REFERENCES arbeidssoekere (periode_id) ON DELETE CASCADE,
    synk_id       UUID         NOT NULL,
    status        VARCHAR(20)  NOT NULL,
    status_endret TIMESTAMP(6) NOT NULL,
    sist_kjoert   TIMESTAMP(6) NOT NULL,
    antall_feil   INT          NOT NULL
);

CREATE INDEX arbeidssoekere_identitetsnummer_idx ON arbeidssoekere (identitetsnummer);
CREATE INDEX tiltak_synk_status_idx ON tiltak_synk (status, status_endret);
