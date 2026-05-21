create table utgang_hendelser_logg (
  id              BIGSERIAL PRIMARY KEY,
  timestamp       TIMESTAMP(3) NOT NULL,
  type            VARCHAR NOT NULL,
  periode_id      UUID NOT NULL,
  brukertype      VARCHAR NOT NULL,
  opplysninger    VARCHAR[]
);

create index utgang_hendelser_logg_periode_id_idx on utgang_hendelser_logg (periode_id, timestamp);

create type kontroll_status_type as enum ('GODKJENT', 'AVVIST', 'UKJENT');

create table perioder (
  id                          UUID PRIMARY KEY,
  arbeidssoeker_id            BIGINT,
  identitetsnummer            VARCHAR NOT NULL,
  trenger_kontroll            BOOLEAN NOT NULL DEFAULT false,
  stoppet                     BOOLEAN NOT NULL DEFAULT false,
  sist_oppdatert              TIMESTAMP(3) NOT NULL,
  initielle_opplysninger      VARCHAR[],
  gjeldende_opplysninger      VARCHAR[],
  gjeldende_tidspunkt         TIMESTAMP(3),
  forrige_opplysninger        VARCHAR[],
  forrige_tidspunkt           TIMESTAMP(3),
  siste_status                kontroll_status_type NOT NULL DEFAULT 'UKJENT'
);

create index perioder_trenger_kontroll_idx on perioder (trenger_kontroll) WHERE trenger_kontroll = true AND stoppet = false;
create index perioder_sist_oppdatert_idx on perioder (sist_oppdatert) WHERE stoppet = false;

create table kontroll_status_logg (
  id              BIGSERIAL PRIMARY KEY,
  periode_id      UUID NOT NULL REFERENCES perioder(id),
  status          kontroll_status_type NOT NULL,
  tidspunkt       TIMESTAMP(3) NOT NULL,
  opplysninger    VARCHAR[]
);

create index kontroll_status_logg_periode_id_idx on kontroll_status_logg (periode_id, tidspunkt);

