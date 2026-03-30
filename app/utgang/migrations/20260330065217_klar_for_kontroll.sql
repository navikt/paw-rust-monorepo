create table klar_for_kontroll (
  id              BIGSERIAL PRIMARY KEY,
  opplysninger_id BIGINT NOT NULL REFERENCES opplysninger(id) ON DELETE CASCADE
);
