create table periode (
    id                          UUID primary key,
    periode_aktiv               boolean not null,
    periode_startet_timestamp   TIMESTAMP(3) not null,
    periode_startet_brukertype  VARCHAR not null,
    periode_avsluttet_timestamp   TIMESTAMP(3) default null,
    periode_avsluttet_brukertype  VARCHAR default null,
    sist_oppdatert_timestamp    TIMESTAMP(3) not null,
    sist_oppdatert_status       VARCHAR not null
);

create table periode_metadata (
    periode_id          UUID primary key,
    identitetsnummer    VARCHAR not null,
    arbeidssoeker_id    BIGINT not null,
    kafka_key           BIGINT not null
);

create table opplysninger (
    id              BIGSERIAL PRIMARY KEY,
    periode_id      UUID NOT NULL REFERENCES periode_metadata(periode_id) ON DELETE CASCADE,
    kilde           VARCHAR NOT NULL,
    tidspunkt       TIMESTAMP(3) NOT NULL,
    opplysninger    VARCHAR[] NOT NULL
);
