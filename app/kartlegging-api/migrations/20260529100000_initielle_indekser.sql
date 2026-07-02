CREATE INDEX arbeidssoekere_arbeidssoeker_id_idx ON arbeidssoekere (arbeidssoeker_id);
CREATE INDEX arbeidssoekere_identitetsnummer_idx ON arbeidssoekere (identitetsnummer);

CREATE INDEX ledighetsperioder_ledig_siden_idx ON ledighetsperioder (ledig_siden);
CREATE INDEX ledighetsperioder_periode_id_idx ON ledighetsperioder (periode_id);
CREATE INDEX ledighetsperioder_periode_startet_idx ON ledighetsperioder (periode_startet);

CREATE INDEX kontortilknytninger_parent_id_idx ON kontortilknytninger (parent_id);
CREATE INDEX kontortilknytninger_kontor_id_idx ON kontortilknytninger (kontor_id);
CREATE INDEX kontortilknytninger_kontor_type_idx ON kontortilknytninger (kontor_type);