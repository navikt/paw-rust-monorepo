create table periode (
    id                          UUID primary key,
    periode_aktiv               boolean not null,
    periode_startet_timestamp   TIMESTAMPZ(3) not null,
    periode_startet_brukertype  VARCHAR not null,
    periode_stoppet_timestamp   TIMESTAMPZ(3) default null,
    periode_stoppet_brukertype  VARCHAR default null,
    sist_oppdatert_timestamp    TIMESTAMPZ(3) not null,
    sist_oppdaterte_status      VARCHAR not null
);

create table periode_metdata (
    periode_id          UUID primary key,
    identitetsnummer    VARCHAR not null,
    arbeidssoeker_id    BIGINT not null,
    kafka_key           BIGINT not null
);

create table opplysninger (
    id              BIGSERIAL PRIMARY KEY,
    periode_id      UUID NOT NULL REFERENCES periode_metdata(id) ON DELETE CASCADE,
    kilde           VARCHAR NOT NULL,
    tidspunkt       TIMESTAMPZ(3) NOT NULL,
    opplysninger    VARCHAR[] NOT NULL
);