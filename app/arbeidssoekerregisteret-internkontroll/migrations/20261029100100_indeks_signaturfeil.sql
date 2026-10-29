-- Visualiseringen henter de siste records som feilet signaturvalidering.
-- Predikatet må være identisk med filteret i hent_siste_signaturfeil.
-- Bygges i migreringens transaksjon: blokkerer skriving til record_derived
-- mens den bygges, men rulles tilbake rent om podden stoppes underveis.
CREATE INDEX record_derived_signaturfeil_idx
    ON record_derived (xid)
    WHERE signature_status NOT IN ('valid', 'unsigned_before_required');
