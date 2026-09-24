-- Støtter DISTINCT ON (arbeidssoeker_id)-oppslaget av aktiv periode i
-- ledighetsperiode_v2::select_by_arbeidssoeker_ids.
CREATE INDEX kartlegging_arbeidssoeker_id_aktiv_idx
    ON kartlegginger (arbeidssoeker_id, arbeidsledig_fra DESC, arbeidssoeker_fra DESC)
    WHERE arbeidssoeker_til IS NULL;

-- Støtter DISTINCT ON (periode_id)-oppslaget av siste egenvurdering/bekreftelse per
-- periode i ledighetsperiode_v2::select_by_arbeidssoeker_ids.
CREATE INDEX egenvurderinger_periode_id_tidspunkt_idx
    ON egenvurderinger (periode_id, tidspunkt DESC);

CREATE INDEX bekreftelser_periode_id_gjelder_til_idx
    ON bekreftelser (periode_id, gjelder_til DESC);
