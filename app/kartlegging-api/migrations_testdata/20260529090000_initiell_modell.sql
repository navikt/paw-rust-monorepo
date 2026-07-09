CREATE TABLE arbeidssoekere
(
    id               BIGSERIAL PRIMARY KEY,
    arbeidssoeker_id BIGINT       NOT NULL,
    identitetsnummer CHAR(11)     NOT NULL,
    fornavn          VARCHAR(255) NOT NULL,
    mellomnavn       VARCHAR(255),
    etternavn        VARCHAR(255) NOT NULL
);

CREATE TABLE kartlegginger
(
    id                  BIGSERIAL PRIMARY KEY,
    parent_id           BIGINT       NOT NULL,
    periode_id          UUID         NOT NULL,
    arbeidssoeker_siden TIMESTAMP(6) NOT NULL,
    arbeidsledig_siden  TIMESTAMP(6),
    FOREIGN KEY (parent_id) REFERENCES arbeidssoekere (id) ON DELETE CASCADE
);

CREATE TABLE perioder
(
    id                  UUID PRIMARY KEY,
    identitetsnummer    CHAR(11)     NOT NULL,
    startet_tidspunkt   TIMESTAMP(6) NOT NULL,
    avsluttet_tidspunkt TIMESTAMP(6)
);

CREATE TABLE opplysninger
(
    id            UUID PRIMARY KEY,
    periode_id    UUID         NOT NULL,
    jobbsituasjon VARCHAR[]    NOT NULL DEFAULT '{}',
    tidspunkt     TIMESTAMP(6) NOT NULL
);

CREATE TABLE profileringer
(
    id              UUID PRIMARY KEY,
    periode_id      UUID         NOT NULL,
    opplysninger_id UUID         NOT NULL,
    profilert_til   VARCHAR(30)  NOT NULL,
    tidspunkt       TIMESTAMP(6) NOT NULL
);

CREATE TABLE egenvurderinger
(
    id              UUID PRIMARY KEY,
    periode_id      UUID         NOT NULL,
    profilering_id  UUID         NOT NULL,
    profilert_til   VARCHAR(30)  NOT NULL,
    egenvurdert_til VARCHAR(30)  NOT NULL,
    tidspunkt       TIMESTAMP(6) NOT NULL
);

CREATE TABLE bekreftelser
(
    id                   UUID PRIMARY KEY,
    periode_id           UUID         NOT NULL,
    gjelder_fra          TIMESTAMP(6) NOT NULL,
    gjelder_til          TIMESTAMP(6) NOT NULL,
    har_jobbet           BOOLEAN      NOT NULL,
    vil_fortsette        BOOLEAN      NOT NULL,
    bekreftelsesloesning VARCHAR(30)  NOT NULL,
    tidspunkt            TIMESTAMP(6) NOT NULL
);

CREATE TABLE bekreftelse_paa_vegne_av
(
    periode_id             UUID PRIMARY KEY,
    bekreftelsesloesninger VARCHAR[] NOT NULL DEFAULT '{}'
);

CREATE TABLE kontortilknytninger
(
    id          BIGSERIAL PRIMARY KEY,
    parent_id   BIGINT       NOT NULL,
    kontor_id   VARCHAR(30)  NOT NULL,
    kontor_navn VARCHAR(255) NOT NULL,
    kontor_type VARCHAR(30)  NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES arbeidssoekere (id) ON DELETE CASCADE
);