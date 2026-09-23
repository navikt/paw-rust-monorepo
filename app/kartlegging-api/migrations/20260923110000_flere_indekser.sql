CREATE INDEX opplysninger_tidspunkt_idx ON opplysninger (tidspunkt);

CREATE INDEX profileringer_tidspunktd_idx ON profileringer (tidspunkt);

CREATE INDEX egenvurderinger_tidspunkt_idx ON egenvurderinger (tidspunkt);

CREATE INDEX bekreftelser_gjelder_fra_idx ON bekreftelser (gjelder_fra);
CREATE INDEX bekreftelser_gjelder_til_idx ON bekreftelser (gjelder_til);
CREATE INDEX bekreftelser_tidspunkt_idx ON bekreftelser (tidspunkt);