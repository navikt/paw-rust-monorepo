create table utgang_hendelser_logg (
  id              BIGSERIAL PRIMARY KEY,
  timestamp       TIMESTAMP(3) NOT NULL,
  type            VARCHAR NOT NULL,
  periode_id      UUID NOT NULL,
  brukertype      VARCHAR NOT NULL,
  opplysninger    VARCHAR[]
);

create index utgang_hendelser_logg_periode_id_idx on utgang_hendelser_logg (periode_id, timestamp);


create table perioder (
  id                  UUID PRIMARY KEY,
  arbeidssoeker_id    BIGINT,
  trenger_kontroll    BOOLEAN NOT NULL,
  sist_oppdatert      TIMESTAMP(3) NOT NULL
);

create index perioder_trenger_kontroll_idx on perioder (trenger_kontroll);
create index perioder_sist_oppdatert_idx on perioder (sist_oppdatert);

