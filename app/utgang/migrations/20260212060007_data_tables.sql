CREATE TABLE aktive_perioder
(
    id                  BIGSERIAL PRIMARY KEY,
    periode_id          UUID NOT NULL UNIQUE,
    identitetsnummer    VARCHAR NOT NULL,
    startet_tidspunkt   TIMESTAMP NOT NULL,    
    startet_brukertype  VARCHAR(50) NOT NULL,
    stoppet_tidspunkt   TIMESTAMP DEFAULT NULL,
    stoppet_brukertype  VARCHAR(50) DEFAULT NULL
);

CREATE TABLE TILSTAND
(
    id                  BIGSERIAL PRIMARY KEY,
    periode_id          UUID NOT NULL,
    kilde               VARCHAR NOT NULL,
    arbeidssoeker_id    BIGINT NOT NULL,
    tidspunkt           TIMESTAMP,
    tilstand            VARCHAR[] NOT NULL
)


