create table perioder (
  id                       UUID PRIMARY KEY,
  arbeidssoeker_id         BIGINT,
  identitetsnummer         VARCHAR NOT NULL,
  stoppet                  JSONB,
  sist_oppdatert           TIMESTAMP(3) NOT NULL,
  trenger_kontroll         BOOLEAN NOT NULL DEFAULT false,
  siste_kontroll_tidspunkt TIMESTAMP(3),
  tilstand                 JSONB
);

create index perioder_trenger_kontroll_idx
  on perioder (sist_oppdatert)
  WHERE trenger_kontroll = true
    AND tilstand IS NOT NULL
    AND stoppet IS NULL;

create index perioder_rekontroll_idx
  on perioder (siste_kontroll_tidspunkt)
  WHERE stoppet IS NULL
    AND tilstand IS NOT NULL;
