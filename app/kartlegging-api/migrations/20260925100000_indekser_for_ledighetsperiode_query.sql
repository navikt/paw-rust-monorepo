-- Støtter DISTINCT ON (periode_id)-oppslaget av siste opplysninger/profilering per
-- periode i ledighetsperiode::select_by_arbeidssoeker_ids. Tilsvarende indekser for
-- egenvurderinger og bekreftelser ble lagt til i 20260924070000.
CREATE INDEX opplysninger_periode_id_tidspunkt_idx
    ON opplysninger (periode_id, tidspunkt DESC);

CREATE INDEX profileringer_periode_id_tidspunkt_idx
    ON profileringer (periode_id, tidspunkt DESC);
