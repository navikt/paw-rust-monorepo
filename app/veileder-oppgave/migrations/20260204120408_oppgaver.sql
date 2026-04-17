CREATE TABLE oppgaver
(
    id                 BIGSERIAL PRIMARY KEY,
    type               VARCHAR(50) NOT NULL,
    status             VARCHAR(50) NOT NULL,
    melding_id         UUID        NOT NULL,
    opplysninger       VARCHAR(50)[] NOT NULL,
    arbeidssoeker_id   BIGINT      NOT NULL,
    identitetsnummer   VARCHAR(11) NOT NULL,
    ekstern_oppgave_id BIGINT,
    tidspunkt          TIMESTAMP   NOT NULL
);

CREATE TABLE oppgave_hendelse_logg
(
    id         BIGSERIAL PRIMARY KEY,
    oppgave_id BIGINT REFERENCES oppgaver (id) ON DELETE CASCADE,
    status     VARCHAR(50) NOT NULL,
    melding    TEXT        NOT NULL,
    tidspunkt  TIMESTAMP   NOT NULL
);

CREATE INDEX oppgaver_status_idx ON oppgaver (status);
CREATE INDEX oppgaver_arbeidssoeker_id_idx ON oppgaver (arbeidssoeker_id);
CREATE INDEX oppgaver_identitetsnummer_idx ON oppgaver (identitetsnummer);
CREATE INDEX oppgaver_ekstern_oppgave_id_idx ON oppgaver (ekstern_oppgave_id);
CREATE INDEX oppgave_hendelse_logg_oppgave_id_idx ON oppgave_hendelse_logg (oppgave_id);
