CREATE TABLE avvist_meldinger
(
    melding_id       UUID PRIMARY KEY,
    aarsak           TEXT        NOT NULL,
    identitetsnummer VARCHAR(11) NOT NULL,
    arbeidssoeker_id BIGINT      NOT NULL,
    tidspunkt        TIMESTAMP   NOT NULL
);

CREATE TABLE avvist_melding_status_logg
(
    melding_id UUID REFERENCES avvist_meldinger(melding_id) ON DELETE CASCADE,
    status     VARCHAR(50) NOT NULL,
    tidspunkt  TIMESTAMP   NOT NULL,
    PRIMARY KEY (melding_id, tidspunkt)
);
