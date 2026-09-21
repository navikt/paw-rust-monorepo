UPDATE kartlegginger
SET arbeidsledig_fra = NULL
WHERE arbeidsledig_fra IS NOT NULL;

UPDATE hwm
SET hwm = -1
WHERE version = 1
  AND topic = 'paw.arbeidssoker-bekreftelse-v1';
