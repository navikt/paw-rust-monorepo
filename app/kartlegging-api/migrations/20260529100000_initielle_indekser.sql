CREATE INDEX arbeidssoekere_arbeidssoeker_id_idx ON arbeidssoekere (arbeidssoeker_id);
CREATE INDEX arbeidssoekere_identitetsnummer_idx ON arbeidssoekere (identitetsnummer);
CREATE INDEX arbeidssoekere_ledig_siden_idx ON arbeidssoekere (ledig_siden);
CREATE INDEX arbeidssoekere_periode_id_idx ON arbeidssoekere (periode_id);
CREATE INDEX arbeidssoekere_periode_startet_idx ON arbeidssoekere (periode_startet);

CREATE INDEX tilknyttet_kontor_parent_id_idx ON tilknyttet_kontor (parent_id);
CREATE INDEX tilknyttet_kontor_kontor_id_idx ON tilknyttet_kontor (kontor_id);
CREATE INDEX tilknyttet_kontor_kontor_type_idx ON tilknyttet_kontor (kontor_type);

-- V2
CREATE INDEX arbeidssoekere_v2_arbeidssoeker_id_idx ON arbeidssoekere_v2 (arbeidssoeker_id);
CREATE INDEX arbeidssoekere_v2_identitetsnummer_idx ON arbeidssoekere_v2 (identitetsnummer);

CREATE INDEX ledighetsperioder_ledig_siden_idx ON ledighetsperioder (ledig_siden);
CREATE INDEX ledighetsperioder_periode_id_idx ON ledighetsperioder (periode_id);
CREATE INDEX ledighetsperioder_periode_startet_idx ON ledighetsperioder (periode_startet);

CREATE INDEX kontortilknytninger_parent_id_idx ON kontortilknytninger (parent_id);
CREATE INDEX kontortilknytninger_kontor_id_idx ON kontortilknytninger (kontor_id);
CREATE INDEX kontortilknytninger_kontor_type_idx ON kontortilknytninger (kontor_type);