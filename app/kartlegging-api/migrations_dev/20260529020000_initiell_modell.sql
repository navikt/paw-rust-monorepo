CREATE TABLE arbeidssoekere
(
    id                 BIGINT PRIMARY KEY, -- arbeidssoeker_id
    aktor_id           VARCHAR(20)  NOT NULL,
    identitetsnummer   VARCHAR(20)  NOT NULL,
    fornavn            VARCHAR(255),
    mellomnavn         VARCHAR(255),
    etternavn          VARCHAR(255),
    inserted_timestamp TIMESTAMP(6) NOT NULL,
    updated_timestamp  TIMESTAMP(6)
);

CREATE TABLE kartlegginger
(
    periode_id         UUID PRIMARY KEY,
    arbeidssoeker_id   BIGINT       NOT NULL,
    arbeidssoeker_fra  TIMESTAMP(6) NOT NULL,
    arbeidssoeker_til  TIMESTAMP(6),
    arbeidsledig_fra   TIMESTAMP(6),
    inserted_timestamp TIMESTAMP(6) NOT NULL,
    updated_timestamp  TIMESTAMP(6),
    CONSTRAINT arbeidssoeker_id_fk FOREIGN KEY (arbeidssoeker_id) REFERENCES arbeidssoekere (id)
);

CREATE TABLE perioder
(
    id                  UUID PRIMARY KEY, -- periode_id
    identitetsnummer    VARCHAR(20)  NOT NULL,
    startet_tidspunkt   TIMESTAMP(6) NOT NULL,
    avsluttet_tidspunkt TIMESTAMP(6),
    inserted_timestamp  TIMESTAMP(6) NOT NULL,
    updated_timestamp   TIMESTAMP(6)
);

CREATE TABLE opplysninger
(
    id                 UUID PRIMARY KEY, -- opplysninger_id
    periode_id         UUID         NOT NULL,
    jobbsituasjon      VARCHAR[]    NOT NULL DEFAULT '{}',
    tidspunkt          TIMESTAMP(6) NOT NULL,
    inserted_timestamp TIMESTAMP(6) NOT NULL,
    updated_timestamp  TIMESTAMP(6)
);

CREATE TABLE profileringer
(
    id                 UUID PRIMARY KEY, -- profilering_id
    periode_id         UUID         NOT NULL,
    opplysninger_id    UUID         NOT NULL,
    profilert_til      VARCHAR(30)  NOT NULL,
    tidspunkt          TIMESTAMP(6) NOT NULL,
    inserted_timestamp TIMESTAMP(6) NOT NULL,
    updated_timestamp  TIMESTAMP(6)
);

CREATE TABLE egenvurderinger
(
    id                 UUID PRIMARY KEY, -- egenvurdering_id
    periode_id         UUID         NOT NULL,
    profilering_id     UUID         NOT NULL,
    profilert_til      VARCHAR(30)  NOT NULL,
    egenvurdert_til    VARCHAR(30)  NOT NULL,
    tidspunkt          TIMESTAMP(6) NOT NULL,
    inserted_timestamp TIMESTAMP(6) NOT NULL,
    updated_timestamp  TIMESTAMP(6)
);

CREATE TABLE bekreftelser
(
    id                   UUID PRIMARY KEY, -- bekreftelse_id
    periode_id           UUID         NOT NULL,
    gjelder_fra          TIMESTAMP(6) NOT NULL,
    gjelder_til          TIMESTAMP(6) NOT NULL,
    har_jobbet           BOOLEAN      NOT NULL,
    vil_fortsette        BOOLEAN      NOT NULL,
    bekreftelsesloesning VARCHAR(50)  NOT NULL,
    tidspunkt            TIMESTAMP(6) NOT NULL,
    inserted_timestamp   TIMESTAMP(6) NOT NULL,
    updated_timestamp    TIMESTAMP(6)
);

CREATE TABLE bekreftelse_paavegneav
(
    periode_id             UUID PRIMARY KEY,
    bekreftelsesloesninger VARCHAR[]    NOT NULL DEFAULT '{}',
    inserted_timestamp     TIMESTAMP(6) NOT NULL,
    updated_timestamp      TIMESTAMP(6)
);

CREATE TABLE kontortilknytninger
(
    id                 UUID PRIMARY KEY, -- oppfolgingsperiode_uuid
    aktor_id           VARCHAR(20)  NOT NULL,
    identitetsnummer   VARCHAR(20)  NOT NULL,
    kontor_id          VARCHAR(30)  NOT NULL,
    kontor_navn        VARCHAR(255) NOT NULL,
    kontor_type        VARCHAR(30)  NOT NULL,
    tidspunkt          TIMESTAMP(6) NOT NULL,
    inserted_timestamp TIMESTAMP(6) NOT NULL,
    updated_timestamp  TIMESTAMP(6)
);