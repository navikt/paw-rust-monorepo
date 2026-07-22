CREATE INDEX arbeidssoekere_aktor_id_idx ON arbeidssoekere (aktor_id);
CREATE INDEX arbeidssoekere_identitetsnummer_idx ON arbeidssoekere (identitetsnummer);

CREATE INDEX kartlegging_periode_id_idx ON kartlegginger (periode_id);
CREATE INDEX kartlegging_arbeidssoeker_id_idx ON kartlegginger (arbeidssoeker_id);
CREATE INDEX kartlegging_arbeidssoeker_fra_idx ON kartlegginger (arbeidssoeker_fra);
CREATE INDEX kartlegging_arbeidsledig_fra_idx ON kartlegginger (arbeidsledig_fra);

CREATE INDEX opplysninger_periode_id_idx ON opplysninger (periode_id);

CREATE INDEX profileringer_periode_id_idx ON profileringer (periode_id);

CREATE INDEX egenvurderinger_periode_id_idx ON egenvurderinger (periode_id);

CREATE INDEX bekreftelser_periode_id_idx ON bekreftelser (periode_id);

CREATE INDEX kontortilknytninger_aktor_id_idx ON kontortilknytninger (aktor_id);
CREATE INDEX kontortilknytninger_identitetsnummer_idx ON kontortilknytninger (identitetsnummer);
CREATE INDEX kontortilknytninger_kontor_id_idx ON kontortilknytninger (kontor_id);
CREATE INDEX kontortilknytninger_kontor_type_idx ON kontortilknytninger (kontor_type);