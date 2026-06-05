CREATE TABLE arbeidssoekere
(
    id                        BIGSERIAL PRIMARY KEY,
    arbeidssoeker_id          BIGINT       NOT NULL,
    identitetsnummer          CHAR(11)     NOT NULL,
    fornavn                   VARCHAR(255) NOT NULL,
    mellomnavn                VARCHAR(255),
    etternavn                 VARCHAR(255) NOT NULL,
    ledig_siden               TIMESTAMP(6),
    periode_id                UUID         NOT NULL,
    periode_startet           TIMESTAMP(6) NOT NULL,
    periode_avsluttet         TIMESTAMP(6),
    opplysninger_id           UUID,
    opplysninger_tidspunkt    TIMESTAMP(6),
    profilering_id            UUID,
    profilert_til             VARCHAR(30),
    profilering_tidspunkt     TIMESTAMP(6),
    egenvurdering_id          UUID,
    egenvurdert_til           VARCHAR(30),
    egenvurdering_tidspunkt   TIMESTAMP(6),
    bekreftelse_id            UUID,
    bekreftelse_gjelder_fra   TIMESTAMP(6),
    bekreftelse_gjelder_til   TIMESTAMP(6),
    bekreftelse_har_jobbet    BOOLEAN,
    bekreftelse_vil_fortsette BOOLEAN,
    bekreftelsesloesning      VARCHAR(30),
    bekreftelse_paa_vegne_av  VARCHAR[]    NOT NULL DEFAULT '{}',
    inserted_timestamp        TIMESTAMP(6) NOT NULL, -- Tidspunkt rad ble opprettet
    updated_timestamp         TIMESTAMP(6)           -- Tidspunkt rad ble endret
);

CREATE TABLE tilknyttet_kontor
(
    id          BIGSERIAL PRIMARY KEY,
    parent_id   BIGINT       NOT NULL,
    kontor_id   VARCHAR(30)  NOT NULL,
    kontor_navn VARCHAR(255) NOT NULL,
    kontor_type VARCHAR(30)  NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES arbeidssoekere (id) ON DELETE CASCADE
);