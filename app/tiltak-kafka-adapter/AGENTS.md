# Plan: synkronisering av tiltakshistorikk (tiltak-kafka-adapter)

## Problem og tilnærming

Appen har en periodekonsument og en arbeidsliste i Postgres. Den skal:

1. Konsumere `paw.arbeidssokerperioder-v1` og holde en arbeidsliste med aktive perioder i Postgres.
2. Kjøre en skedulert jobb hvert 10. minutt som synkroniserer hver arbeidssøker. Vellykket synkroniserte arbeidssøkere blir klare for ny synkronisering etter 2 timer.
3. Hente alle identer fra `paw_kafka_key_gen` og tiltakshistorikk fra Mulighetsrommet.
4. (Senere) Produsere Kafka-melding og oppdatere status i samme logiske operasjon.

Arkitekturen skal tåle flere instanser. Instansene koordinerer gjennom `tiltak_synk` i Postgres.

## Arkitektur

| Område | Valg | Begrunnelse |
|---|---|---|
| Skedulering | `tokio::time::interval` med `MissedTickBehavior::Delay` | Samme mønster som `veileder-oppgave`. Ingen ny avhengighet. Kjøringer i samme instans overlapper aldri. |
| Koordinering mellom instanser | `VENTER` → `AKTIV` med `FOR UPDATE SKIP LOCKED`, `synk_id` som vakt ved statusoppdatering | To instanser får disjunkte batcher. Trenger ikke leader election eller distribuert lås. |
| Kilde til arbeidslisten | HWM-konsument (`paw_kafka::hwm`) på periode-topic, Avro via schema registry, `eksterne_hendelser::Periode` | Samme mønster som `kartlegging-api`. |
| Identer | `paw_key_gen_client::finn_identiteter`, send alle `FOLKEREGISTERIDENT` (gjeldende og historiske) | API-et krever historiske identer for komplett historikk. |
| Auth mot Mulighetsrommet og key-gen | Entra M2M via `texas_client` | Avklart med Mulighetsrommet. |
| Ukjente enum-verdier | `Ukjent(String)` | Nye tiltakstyper skal ikke stoppe jobben. |
| Lagring av tiltakshistorikk | Ingen | Data går bare videre i Kafka-meldingen. |

### Tabellene `arbeidssoekere` og `tiltak_synk`

```sql
CREATE TABLE arbeidssoekere
(
    periode_id       UUID PRIMARY KEY,
    identitetsnummer VARCHAR(20)  NOT NULL,
    periode_startet  TIMESTAMP(6) NOT NULL
);

CREATE TABLE tiltak_synk
(
    periode_id    UUID PRIMARY KEY REFERENCES arbeidssoekere (periode_id) ON DELETE CASCADE,
    synk_id       UUID         NOT NULL,
    status        VARCHAR(20)  NOT NULL,
    status_endret TIMESTAMP(6) NOT NULL,
    sist_kjoert   TIMESTAMP(6) NOT NULL,
    antall_feil   INT          NOT NULL
);

CREATE INDEX arbeidssoekere_identitetsnummer_idx ON arbeidssoekere (identitetsnummer);
CREATE INDEX tiltak_synk_status_idx ON tiltak_synk (status, status_endret);
```

- Konsumenten skriver bare til `arbeidssoekere`. Init-steget i jobben oppretter manglende rader i `tiltak_synk`, både for perioder som allerede ligger i dev og for nye perioder.
- Ved periodeslutt slettes raden i `arbeidssoekere`. Postgres sletter raden i `tiltak_synk` via `ON DELETE CASCADE`.
- En person har aldri mer enn én aktiv periode.

Tidsfeltene er `TIMESTAMP(6)` uten tidssone. Bruk UTC konsekvent (`timezone('UTC', now())`). `sist_kjoert` er tidspunktet for siste kjøring, vellykket eller ikke. Nye rader får `sist_kjoert = 'epoch'`.

## Synkroniseringsjobben

Hver kjøring får en ny `synk_id` (UUIDv4) og kjører fem steg i fast rekkefølge. Et feilet steg logges og stopper ikke resten av kjøringen.

| # | Steg | Overgang | Betingelse |
|---|------|----------|------------|
| 1 | init | (ingen rad) → `VENTER` | periode i `arbeidssoekere` uten rad i `tiltak_synk` |
| 2 | feilet | `FEILET` → `VENTER` | `antall_feil < maks_feil_per_rad` (10) |
| 2 | feilet | `FEILET` → `AVBRUTT` | `antall_feil >= maks_feil_per_rad` |
| 2 | feilet | `AKTIV` → `VENTER` | `status_endret < now - synk_ventetid` (krasjet instans) |
| 3 | avbrutt | `AVBRUTT` → `VENTER`, `antall_feil = 0` | `sist_kjoert < now - synk_ventetid` (2 timer) |
| 4 | ferdig | `FERDIG` → `VENTER`, `antall_feil = 0` | `sist_kjoert < now - synk_ventetid` (2 timer) |
| 5 | normal | `VENTER` → `AKTIV` → `FERDIG`/`FEILET` | batcher på `batch_stoerrelse` (100) til ingen `VENTER` gjenstår |

Steg 1–4 er idempotente og trygge å kjøre fra flere instanser samtidig.

Konsekvenser:

- `FEILET` prøves på nytt ved hver kjøring, altså hvert 10. minutt, i opptil 10 forsøk. Deretter blir raden `AVBRUTT` i 2 timer og får 10 nye forsøk.
- `FEILET` → `VENTER` nullstiller ikke `antall_feil`. Ellers når raden aldri grensen og blir aldri `AVBRUTT`.

### SQL

Steg 1, init:

```sql
INSERT INTO tiltak_synk (periode_id, synk_id, status, status_endret, sist_kjoert, antall_feil)
SELECT a.periode_id, $1, 'VENTER', timezone('UTC', now()), 'epoch'::timestamp, 0
FROM arbeidssoekere a
WHERE NOT EXISTS (SELECT 1 FROM tiltak_synk t WHERE t.periode_id = a.periode_id)
ON CONFLICT (periode_id) DO NOTHING;
```

Steg 2, feilet (én transaksjon):

```sql
UPDATE tiltak_synk SET status = 'AVBRUTT', status_endret = timezone('UTC', now())
WHERE status = 'FEILET' AND antall_feil >= $1;
UPDATE tiltak_synk SET status = 'VENTER', status_endret = timezone('UTC', now())
WHERE status = 'FEILET' AND antall_feil < $1;
UPDATE tiltak_synk SET status = 'VENTER', status_endret = timezone('UTC', now())
WHERE status = 'AKTIV' AND status_endret < timezone('UTC', now()) - $2::interval;
```

Steg 3 og 4, avbrutt og ferdig (`$1` = status):

```sql
UPDATE tiltak_synk
SET status = 'VENTER', status_endret = timezone('UTC', now()), antall_feil = 0
WHERE status = $1 AND sist_kjoert < timezone('UTC', now()) - $2::interval;
```

Steg 5, claim av én batch:

```sql
UPDATE tiltak_synk t
SET status = 'AKTIV', synk_id = $1, status_endret = timezone('UTC', now())
FROM arbeidssoekere a
WHERE a.periode_id = t.periode_id
  AND t.periode_id IN (
    SELECT periode_id FROM tiltak_synk
    WHERE status = 'VENTER'
    ORDER BY sist_kjoert
    LIMIT $2
    FOR UPDATE SKIP LOCKED)
RETURNING t.periode_id, a.identitetsnummer;
```

Steg 5, resultat per rad (vakt på `synk_id` og `AKTIV`):

```sql
-- ok
UPDATE tiltak_synk
SET status = 'FERDIG', status_endret = timezone('UTC', now()), sist_kjoert = timezone('UTC', now()), antall_feil = 0
WHERE periode_id = $1 AND synk_id = $2 AND status = 'AKTIV';
-- feil
UPDATE tiltak_synk
SET status = 'FEILET', status_endret = timezone('UTC', now()), sist_kjoert = timezone('UTC', now()), antall_feil = antall_feil + 1
WHERE periode_id = $1 AND synk_id = $2 AND status = 'AKTIV';
```

0 rader påvirket betyr at perioden er slettet eller raden er tatt over. Jobben logger det på debug med `periode_id` og teller det ikke som feil.

### Normal synk per arbeidssøker

1. `paw_key_gen_client::finn_identiteter`, filtrer `Folkeregisterident` (gjeldende og historiske).
2. `mulighetsrommet_client::hent_historikk(identer)`. Ett kall per arbeidssøker.
3. `TiltakshistorikkPublisher::publiser(periode_id, historikk)`. Første implementasjon logger antall deltakelser.
4. Oppdater raden til `FERDIG` eller `FEILET`. Alle feiltyper fra key-gen, Mulighetsrommet og publisher gir `FEILET`, også 401 og 403.

Jobben venter `request_grace_time` før hvert utgående HTTP-kall (rate limit) og `batch_grace_time` før neste batch. Løkken stopper når claim returnerer 0 rader.

### Avbrudd av kjøringen

Telleren for feil per kjøring er en lokal variabel i jobben, ikke en kolonne. Den starter på 0 for hver kjøring og hver instans, og teller alle feil i steg 5.

Når telleren når `maks_feil_per_kjoering` (10):

1. Stopp behandlingen av batchen og hent ingen nye batcher.
2. Sett gjenværende `AKTIV`-rader med denne `synk_id` tilbake til `VENTER` uten å øke `antall_feil`. Neste kjøring tar dem.
3. Logg `synk_id` og antall rader som ble satt tilbake.

```sql
UPDATE tiltak_synk SET status = 'VENTER', status_endret = timezone('UTC', now())
WHERE synk_id = $1 AND status = 'AKTIV';
```

`antall_feil` i `tiltak_synk` er en egen teller per rad over flere kjøringer. Den styrer `FEILET` → `AVBRUTT`.

### Kafka og database (designes senere)

DB og Kafka kan ikke dele en atomisk transaksjon, og outbox er uaktuelt fordi historikken ikke skal lagres. Forslaget er å sende til Kafka og vente på leveringsbekreftelse, og deretter commite status `FERDIG`. Det gir at-least-once, så konsumentene må tåle duplikater. Perioden kan bli avsluttet mens raden er `AKTIV`. DB-vakten stopper da statusoppdateringen, men Kafka-meldingen kan alt være sendt. Begge deler avklares i designet av Kafka-produksjon. Frem til da kaller synkroniseringen en `TiltakshistorikkPublisher`-trait med en implementasjon som bare logger antall deltakelser.

### Config (`app_config.toml`, lokal og nais)

```toml
synk_task_interval = "PT10M"
synk_ventetid = "PT2H"
batch_stoerrelse = 100
batch_grace_time = "PT0S"
request_grace_time = "PT0S"
maks_feil_per_rad = 10
maks_feil_per_kjoering = 10

[mulighetsrommet]   # placeholder, utvikleren fyller inn
base_url = "http://TODO"
scope = "api://TODO/.default"

[key_gen]
base_url = "http://paw-kafka-key-generator"
scope = "api://dev-gcp.paw.paw-kafka-key-generator/.default"
```

Durations leses med `duration::iso8601::deserialize` til `std::time::Duration`. Klientconfig følger `MulighetsrommetClientConfig` og `PawKeyGenClientConfig`.

### Kodestruktur

- `src/model/dao/tiltak_synk.rs`: `init_manglende`, `feilet_til_venter_eller_avbrutt` (med hengende `AKTIV`), `til_venter_etter_ventetid(status)`, `claim_batch`, `sett_ferdig`, `sett_feilet`, `frigi_aktive(synk_id)`.
- `src/logic/synk/publisher.rs`: `trait TiltakshistorikkPublisher` og `LoggPublisher`.
- `src/logic/synk/service.rs`: `SynkService` med `kjoer(synk_id)` som kjører de fem stegene.
- `src/logic/synk/task.rs`: interval-løkke som kaller `kjoer`.
- `src/config.rs`, `main.rs`: config, `texas_client`, klienter og task i `tokio::select!`.
- `Cargo.toml`: `paw_key_gen_client`, `mulighetsrommet_client`, `texas_client`.

### Observerbarhet

- `tiltakshistorikk_synkronisert_total`, `tiltakshistorikk_feilet_total`, `tiltakshistorikk_avbrutt_total` (counters)
- `arbeidssoekere_antall{status}` (gauge)
- `arbeidssoekere_eldste_synkronisering_sekunder` (gauge)
- `arbeidssoekere_vedvarende_feil` (gauge, rader med `antall_feil >= 5`), med varsel i Grafana
- `#[instrument]` på kjøring (`synk_id`) og per arbeidssøker (`periode_id`). Ingen fnr i logger eller spans.

### Nais

- `gcp.sqlInstances` (Postgres), `kafka.pool`, `azure.application.enabled: true`
- `accessPolicy.outbound`: Utvikleren legger til policy for Mulighetsrommet manuelt.
- `isReady` skal vise status for database og konsument.
- `replicas.max: 2` i dev, så oppsettet med flere instanser blir testet.
- Ingen CPU-limits. Mål minnebruken og juster fra 32Mi.
- Scope i dev: `api://dev-gcp.paw.paw-kafka-key-generator/.default` (som i `kartlegging-api`).
- Vurder DB-audit-logging fordi tabellen lagrer fnr. Eksisterende audit-logging skal ikke slås av.

## 🔴 Rød sone: forstå dette grundig

- SQL for claim, gjenoppretting og statusoppdatering med `synk_id`-vakt (samtidighet mellom instanser)
- Avbruddslogikken (feilteller per kjøring og `AVBRUTT` per rad)
- Rekkefølgen Kafka-sending og DB-commit (når Kafka designes)

Assistenten implementerer SQL for samtidighet og avbruddslogikken. Utvikleren reviewer.

Grønn sone: domenemodell, klient, migrasjoner, Nais, config og metrikk-boilerplate.

## Teststrategi

- `domain/mulighetsrommet`: serde-tester på eksempel-JSON for alle tre deltakelsestyper og for `Ukjent`.
- `lib/mulighetsrommet_client`: mockito, `token_client_stub`, statuskoder 200/401/403/5xx.
- Konsument: aktiv periode oppretter rad i `arbeidssoekere`, gjentatt melding er idempotent, og avsluttet periode sletter begge rader.
- DAO med testcontainers-Postgres:
  - init oppretter rader for perioder uten synk-rad, rører ikke eksisterende rader og er idempotent.
  - feilet: `antall_feil` 9 → `VENTER` med telleren urørt, 10 → `AVBRUTT`. `AKTIV` eldre enn 2 timer → `VENTER`, nyere står.
  - avbrutt og ferdig: eldre enn 2 timer → `VENTER` med `antall_feil = 0`, nyere står.
  - claim: grense på 100, to samtidige claims gir disjunkte sett, bare `VENTER` tas.
  - resultat: ok setter `FERDIG`, `sist_kjoert` og `antall_feil = 0`. Feil øker `antall_feil`. Feil `synk_id` gir 0 rader.
  - frigi: bare `AKTIV`-rader med egen `synk_id` går til `VENTER`.
- Service med mockito: hele kjøringen fra init til `FERDIG`, 401/403/5xx gir `FEILET`, slettet periode under kjøring, og kjøringen stopper ved feil nummer 10 med resten av batchen tilbake i `VENTER`.
- Del aldri en statisk `PgPool` mellom flere `#[tokio::test]`.

## Todos (rekkefølge = commits, hver kan deployes)

1. `domain-mulighetsrommet`: Utført.
2. `lib-mulighetsrommet-client`: Utført.
3. `db-oppsett`: Utført.
4. `periode-konsument`: Utført.
5. `tiltak-synk-repo`: SQL for de fem stegene, frigi og resultat, med tester. 🔴
6. `synk-jobb`: config, klienter, service, task, publisher og feilteller per kjøring, med tester. 🔴 (avbrudd)
7. `metrikker`: counters og gauges.
8. `nais-config`: Utført, bortsett fra outbound policy til Mulighetsrommet som utvikleren legger inn manuelt.
9. (Senere) `kafka-produksjon`: eget design.

## Åpne punkter

- Utvikleren setter inn endpoint, scope og outbound policy for Mulighetsrommet manuelt før jobben aktiveres.
- Dette er en betaversjon. Sikkerhetsgjennomgangen er utsatt og gjøres før prod.
