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
