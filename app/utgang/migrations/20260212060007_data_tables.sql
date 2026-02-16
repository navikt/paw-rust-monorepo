CREATE TABLE aktive_perioder
(
    id                  BIGSERIAL PRIMARY KEY,
    periode_id          UUID NOT NULL UNIQUE,
    identitetsnummer    VARCHAR NOT NULL,
    startet             TIMESTAMP NOT NULL,
    bruker_type         VARCHAR(50) NOT NULL,
    bruker_id           VARCHAR(50) NOT NULL
    tilstand            VARCHAR[] NOT NULL,
    sist_ok             TIMESTAMP
);

CREATE TABLE GJELDENE_TILSTAND
(
    id                  BIGSERIAL PRIMARY KEY,
    aktive_periode_id   BIGINT REFERENCES aktive_perioder (id) UNIQUE ON DELETE CASCADE,
    tidspunkt           TIMESTAMP,
    tilstand            VARCHAR[] NOT NULL,
    ok                  BOOLEAN NOT NULL
)


