CREATE INDEX arbeidssoekere_identitetsnummer_idx ON arbeidssoekere(identitetsnummer);
CREATE INDEX arbeidssoekere_ledig_siden_idx ON arbeidssoekere(ledig_siden);
CREATE INDEX arbeidssoekere_periode_id_idx ON arbeidssoekere(periode_id);
CREATE INDEX arbeidssoekere_periode_startet_idx ON arbeidssoekere(periode_startet);

CREATE INDEX tilknyttet_kontor_parent_id_idx ON tilknyttet_kontor(parent_id);
CREATE INDEX tilknyttet_kontor_kontor_id_idx ON tilknyttet_kontor(kontor_id);
CREATE INDEX tilknyttet_kontor_kontor_type_idx ON tilknyttet_kontor(kontor_type);