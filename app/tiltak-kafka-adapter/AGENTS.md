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
| Koordinering mellom instanser | `VENTER` → `AKTIV` med `FOR UPDATE SKIP LOCKED` i claim, `synk_id` som vakt ved statusoppdatering | To instanser får disjunkte batcher uten å vente på hverandre. 0 rader fra claim betyr at ingen ledige `VENTER` finnes. Trenger ikke leader election eller distribuert lås. |
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
| 2 | feilet | `AKTIV` → `VENTER` | `status_endret < now - synk_interval` (krasjet instans) |
| 3 | avbrutt | `AVBRUTT` → `VENTER`, `antall_feil = 0` | `sist_kjoert < now - synk_interval` (2 timer) |
| 4 | ferdig | `FERDIG` → `VENTER`, `antall_feil = 0` | `sist_kjoert < now - synk_interval` (2 timer) |
| 5 | normal | `VENTER` → `AKTIV` → `FERDIG`/`FEILET` | batcher på `batch_size` (100) til ingen `VENTER` gjenstår |

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

Steg 2–4 og frigi bruker samme mal i `update_status` (`update_status_sql!`). `$1` er verdien til `Betingelse`, `$2` er ny status, `$3` er `nullstill_feil` og `$4` er gammel status:

```sql
UPDATE tiltak_synk
SET status = $2, status_endret = timezone('UTC', now()),
    antall_feil = CASE WHEN $3 THEN 0 ELSE antall_feil END
WHERE status = $4 AND <betingelse>;
```

| `Betingelse` | SQL |
|---|---|
| `AntallFeilMinst(n)` | `antall_feil >= $1` |
| `AntallFeilUnder(n)` | `antall_feil < $1` |
| `StatusEndretFoer(t)` | `status_endret < ($1 AT TIME ZONE 'UTC')` |
| `SistKjoertFoer(t)` | `sist_kjoert < ($1 AT TIME ZONE 'UTC')` |
| `SynkId(id)` | `synk_id = $1` |

Tidsgrensen `t` regnes ut i Rust med `SynkConfig::synk_interval_grensed()` (`Utc::now() - synk_interval`) og bindes som `DateTime<Utc>`.

| Steg | `fra` → `til` | `Betingelse` | `nullstill_feil` |
|---|---|---|---|
| 2 | `FEILET` → `AVBRUTT` | `AntallFeilMinst(maks_feil_per_rad)` | `false` |
| 2 | `FEILET` → `VENTER` | `AntallFeilUnder(maks_feil_per_rad)` | `false` |
| 2 | `AKTIV` → `VENTER` | `StatusEndretFoer(grense)` | `false` |
| 3 | `AVBRUTT` → `VENTER` | `SistKjoertFoer(grense)` | `true` |
| 4 | `FERDIG` → `VENTER` | `SistKjoertFoer(grense)` | `true` |
| frigi | `AKTIV` → `VENTER` | `SynkId(synk_id)` | `false` |

De tre oppdateringene i steg 2 kjøres i én transaksjon.

Steg 5, claim av én batch (`FOR UPDATE SKIP LOCKED`):

```sql
WITH kandidater AS (
    SELECT periode_id FROM tiltak_synk
    WHERE status = 'VENTER'
    ORDER BY sist_kjoert, periode_id
    LIMIT $2
    FOR UPDATE SKIP LOCKED
)
UPDATE tiltak_synk t
SET status = 'AKTIV', synk_id = $1, status_endret = timezone('UTC', now())
FROM kandidater k, arbeidssoekere a
WHERE t.periode_id = k.periode_id
  AND a.periode_id = t.periode_id
  AND t.status = 'VENTER'
RETURNING t.periode_id, a.identitetsnummer;
```

- `FOR UPDATE SKIP LOCKED` låser kandidatene og hopper over rader som en annen instans har låst. En claim venter aldri på en annen instans, og to samtidige claims får disjunkte sett.
- Fordi låste rader hoppes over i stedet for å filtreres bort etterpå, fyller claim batchen med neste ledige rader. 0 rader betyr at ingen ledige `VENTER` finnes. Rader som ble hoppet over, holder på å bli `AKTIV` hos en annen instans.
- `AND t.status = 'VENTER'` på den ytre raden beholdes som ekstra vakt. Den koster nesten ingenting.
- `RETURNING` gir nøyaktig radene som fikk jobbens `synk_id`.
- Claim kjøres i en kort, egen transaksjon (`begin`, claim, `commit`). Radlåsene holdes bare i millisekunder.
- Isolasjonsnivå må være `READ COMMITTED` (default).
- Rekkefølgen etter `sist_kjoert` er streng bare per instans. Radene en instans hopper over, tas av den andre.
- `40001` (serialization_failure) og `40P01` (deadlock) behandles som en tom claim, ikke som feil. De teller ikke mot `maks_feil_per_kjoering`. Med `SKIP LOCKED` skal de i praksis ikke oppstå.
- Andre feil fra claim avslutter steg 5 med `slutt = None` i `Kjoering`. De teller ikke mot `maks_feil_per_kjoering`. Frigi kjøres likevel.

Steg 5, resultat per rad (`update_resultat`, vakt på `synk_id` og `AKTIV`, `$3` er `FERDIG` eller `FEILET`):

```sql
UPDATE tiltak_synk
SET status        = $3,
    status_endret = timezone('UTC', now()),
    sist_kjoert   = timezone('UTC', now()),
    antall_feil   = CASE WHEN $3 = 'FERDIG' THEN 0 ELSE antall_feil + 1 END
WHERE periode_id = $1 AND synk_id = $2 AND status = 'AKTIV';
```

0 rader påvirket betyr at perioden er slettet eller raden er tatt over. Jobben logger det på debug med `periode_id` og teller det ikke som feil.

### Normal synk per arbeidssøker

1. `paw_key_gen_client::finn_identiteter`, filtrer `Folkeregisterident` (gjeldende og historiske). Tom liste gir `SynkFeil::IngenIdenter` og `FEILET`.
2. `mulighetsrommet_client::hent_historikk(identer)`. Ett kall per arbeidssøker.
3. `TiltakshistorikkPublisher::publiser(periode_id, historikk)`. Første implementasjon logger antall deltakelser.
4. Oppdater raden til `FERDIG` eller `FEILET`. Alle feiltyper fra key-gen, Mulighetsrommet og publisher gir `FEILET`, også 401 og 403.

Jobben venter `request_grace_time` før hvert utgående HTTP-kall (rate limit) og `batch_grace_time` før neste batch.

Stoppbetingelse for batchløkken:

1. Claim returnerer rader: behandle dem og hent neste batch.
2. Claim returnerer 0 rader: løkken er ferdig (`BatchSlutt::IngenVentende`).
3. Behandleren ber om stopp (feilgrense nådd): løkken er ferdig (`BatchSlutt::Stoppet`).

Det trengs ingen ekstra sjekk mot `VENTER` og ingen teller for tomme claims.

### Avbrudd av kjøringen

Telleren for feil per kjøring er en lokal variabel i jobben, ikke en kolonne. Den starter på 0 for hver kjøring og hver instans, og teller alle feil i steg 5.

Når telleren når `maks_feil_per_kjoering` (10), stopper jobben behandlingen av batchen og henter ingen nye batcher.

Etter steg 5, uansett hvordan løkken endte, settes gjenværende `AKTIV`-rader med denne `synk_id` tilbake til `VENTER` uten å øke `antall_feil`. Det dekker både avbrudd og rader der statusoppdateringen feilet. Jobben logger `synk_id` og antall rader som ble satt tilbake. Neste kjøring tar dem.

Feil i statusoppdateringen teller også mot `maks_feil_per_kjoering`. 0 rader påvirket teller ikke.

Frigi (`frigi_aktive`) bruker malen i `update_status` med `AKTIV` → `VENTER`, `Betingelse::SynkId(synk_id)` og `nullstill_feil = false` (se SQL). Antallet havner i `Kjoering.frigitt` og logges i sluttloggen for kjøringen.

`antall_feil` i `tiltak_synk` er en egen teller per rad over flere kjøringer. Den styrer `FEILET` → `AVBRUTT`.

### Kafka og database (designes senere)

DB og Kafka kan ikke dele en atomisk transaksjon, og outbox er uaktuelt fordi historikken ikke skal lagres. Forslaget er å sende til Kafka og vente på leveringsbekreftelse, og deretter commite status `FERDIG`. Det gir at-least-once, så konsumentene må tåle duplikater. Perioden kan bli avsluttet mens raden er `AKTIV`. DB-vakten stopper da statusoppdateringen, men Kafka-meldingen kan alt være sendt. Begge deler avklares i designet av Kafka-produksjon. Frem til da kaller synkroniseringen en `TiltakshistorikkPublisher`-trait med en implementasjon som bare logger antall deltakelser.

### Config

`app_config.toml` (lokal og nais). Tabellen `[synk]` leses inn i `AppConfig.synk: SynkConfig`. Verdier i nais:

```toml
[synk]
aktivert = true               # false slår av hele synk-tasken
task_interval = "PT10M"
synk_interval = "PT2H"        # ventetid før FERDIG/AVBRUTT og hengende AKTIV går til VENTER
batch_size = 100
batch_grace_time = "PT0S"
request_grace_time = "PT0S"
maks_feil_per_rad = 10
maks_feil_per_kjoering = 10
```

Lokalt er `aktivert = true`, `task_interval = "PT1M"`, `synk_interval = "PT10M"`, `batch_size = 10` og begge feilgrensene 3.

Durations leses med `duration::iso8601::deserialize` til `std::time::Duration`. `SynkConfig::synk_interval_grensed()` gir tidsgrensen `Utc::now() - synk_interval`.

`[kafka]` har `paw_periode_topic` og `paw_arbeidsmarkedstiltak_topic` (`paw.arbeidsmarkedstiltak-beta-v1` i nais). Det siste er klart for todo 11 og brukes ikke ennå.

Klientconfig ligger i egne filer, som i `kartlegging-api`:

| Fil | Nais |
|---|---|
| `token_client_config.toml` | `token_endpoint = "${NAIS_TOKEN_ENDPOINT}"` |
| `paw_key_gen_client_config.toml` | `url = "http://paw-kafka-key-generator"`, `target_scope = "api://${NAIS_CLUSTER_NAME}.paw.paw-kafka-key-generator/.default"` |
| `mulighetsrommet_client_config.toml` | `url = "http://tiltakshistorikk.team-mulighetsrommet.svc.cluster.local"`, `target_scope = "api://${NAIS_CLUSTER_NAME}.team-mulighetsrommet.tiltakshistorikk/.default"` |

Scope bygges fra `NAIS_CLUSTER_NAME`, som Nais setter. Samme mønster som `tilgangskontroll-v2`. Ingen egne scope-variabler i Nais-manifestet.

### Kodestruktur

- `src/model/dao/tiltak_synk.rs`: spørringer mot `tiltak_synk`. Produksjonskoden bruker `insert_manglende` (steg 1), `update_status(fra, til, Betingelse, nullstill_feil)`, `update_resultat(periode_id, synk_id, til)`, `select_batch` (claim) og `select_status_statistikk` (metrikker). `Betingelse` er `AntallFeilMinst`, `AntallFeilUnder`, `StatusEndretFoer`, `SistKjoertFoer` eller `SynkId`. Tidsgrensen beregnes i Rust (`Utc::now() - synk_interval`). `TiltakSynkRow`, `insert`, `select_by_periode_id` og `count_by_status` ligger også her og brukes foreløpig bare av tester.
- `src/logic/service/arbeidssoeker.rs`: `ArbeidssoekerService` med de spissede overgangene, hver i egen transaksjon: `init_manglende`, `prosesser_feilede` (steg 2, med hengende `AKTIV`, returnerer `Gjenoppretting`), `prosesser_avbrutte` (steg 3), `prosesser_ferdige` (steg 4), `sett_ferdig`, `sett_feilet` og `frigi_aktive(synk_id)`.
- `src/logic/batch/mod.rs`: `BatchSlutt` (`IngenVentende` eller `Stoppet`), `Kjoering` og `trait BatchBehandler`. Callbacken er en trait og ikke en `AsyncFnMut`-closure, fordi futures fra async closures ikke blir `Send` og ikke kan kjøres i `tokio::spawn`.
- `src/logic/batch/tiltak_synk.rs`: `for_hver_batch` (batchløkken med stoppbetingelsen) og `TiltakSynkJobb` med `run(synk_id)` som kjører de fem stegene og returnerer `Kjoering`. Steg 5 implementerer `BatchBehandler` i den private structen `Steg5`, som holder feiltelleren.
- `src/kafka/publisher.rs`: `trait TiltakshistorikkPublisher` og `LoggPublisher`.
- `src/logic/batch/task.rs`: interval-løkke som kaller `run`. Første kjøring skjer etter ett helt `task_interval`, ikke ved oppstart. Er `aktivert = false`, logger tasken det og gjør ingenting.
- `tests/support/`: felles testhjelpere (testdata, SQL for å opprette og lese rader, mocks for key-gen og Mulighetsrommet, config). Integrasjonstestene i `tests/batch`, `tests/logic` og `tests/model` kompileres som moduler under `tests/support/test_kafka_producer.rs`.
- `src/config.rs`, `main.rs`: config, `texas_client`, klienter og task i `tokio::select!`.

### Observerbarhet

Navn følger prefikset i kildekoden (`paw_tiltak_kafka_adapter_`). I tillegg kommer Kafka-konsumentmetrikkene.

- `paw_tiltak_kafka_adapter_tiltakshistorikk_total{resultat}` (counter, `src/logic/metrics/tiltak_synk.rs`). `resultat` er `synkronisert`, `feilet` eller `avbrutt`. `behandle_rad` øker `synkronisert` og `feilet` når statusoppdateringen traff en rad. Steg 2 øker `avbrutt` med antall rader som gikk til `AVBRUTT`.
- `paw_tiltak_kafka_adapter_database{type}` (gauge, `src/logic/metrics/database.rs`), oppdatert i `metrics_task`. `arbeidssoekere_total` kommer fra `arbeidssoeker::count`. De andre verdiene kommer fra én spørring, `tiltak_synk::select_status_statistikk` (`GROUP BY status`), og `SynkStatistikk::fra_rader` regner dem ut:
  - `status_venter`, `status_aktiv`, `status_ferdig`, `status_feilet`, `status_avbrutt`: antall rader per status. Statuser uten rader får 0.
  - `eldste_synkronisering_sekunder`: `now - MIN(sist_kjoert)` over alle rader, 0 når tabellen er tom. Nye rader har `epoch` og gir høy verdi til første kjøring.
  - `vedvarende_feil`: rader med `antall_feil >= 5` (`VEDVARENDE_FEIL_GRENSE`). Varsel i Grafana settes opp utenfor repoet.
- `#[instrument]` på kjøring (`synk_id`) og per arbeidssøker (`periode_id`). Ingen fnr i logger eller spans.

### Nais

- `gcp.sqlInstances` (Postgres), `kafka.pool`, `azure.application.enabled: true` (gir `NAIS_TOKEN_ENDPOINT`)
- `accessPolicy.outbound`: `paw-kafka-key-generator` og `team-mulighetsrommet/tiltakshistorikk`.
- `isReady` er alltid `true` (`AppState` uten DB- eller konsumentsjekk). Rebalance-handleren setter bare `isAlive`. Status for database og konsument i `isReady` gjenstår.
- `replicas.max: 1` i dev. Samtidighet mellom instanser testes med testcontainers.
- Ingen CPU-limits. Mål minnebruken og juster fra 32Mi.
- Scope via `NAIS_CLUSTER_NAME` i klientconfig (se Config). Inbound-regler hos key-gen og Mulighetsrommet er lagt inn i dev.
- Vurder DB-audit-logging fordi tabellen lagrer fnr. Ikke konfigurert i `nais-dev.yaml` ennå. Eksisterende audit-logging skal ikke slås av.

## 🔴 Rød sone: forstå dette grundig

- SQL for claim (`FOR UPDATE SKIP LOCKED` med `t.status = 'VENTER'`-vakt), gjenoppretting og statusoppdatering med `synk_id`-vakt (samtidighet mellom instanser)
- Stoppbetingelsen i batchløkken
- Avbruddslogikken (feilteller per kjøring og `AVBRUTT` per rad)
- Rekkefølgen Kafka-sending og DB-commit (når Kafka designes)

Assistenten implementerer SQL for samtidighet og avbruddslogikken. Utvikleren reviewer.

Grønn sone: domenemodell, klient, migrasjoner, Nais, config og metrikk-boilerplate.

## Teststrategi

- `domain/mulighetsrommet`: serde-tester på eksempel-JSON for alle tre deltakelsestyper og for `Ukjent`.
- `lib/mulighetsrommet_client`: mockito, `token_client_stub`, statuskoder 200/401/403/5xx.
- Konsument (mangler, `logic/process/periode.rs` har tom `mod tests`): aktiv periode oppretter rad i `arbeidssoekere`, gjentatt melding er idempotent, og avsluttet periode sletter begge rader.
- DAO med testcontainers-Postgres (`tests/model/tiltak_synk_dao.rs`):
  - init oppretter rader for perioder uten synk-rad, rører ikke eksisterende rader og er idempotent.
  - feilet: `antall_feil` 9 → `VENTER` med telleren urørt, 10 → `AVBRUTT`. `AKTIV` eldre enn 2 timer → `VENTER`, nyere står.
  - avbrutt og ferdig: eldre enn 2 timer → `VENTER` med `antall_feil = 0`, nyere står.
  - claim: grensen respekteres (grense 3 i testen), bare `VENTER` tas, to samtidige claims gir disjunkte sett. En claim mens en annen instans holder ucommittet claim blokkerer ikke og får de resterende ledige radene.
  - resultat: ok setter `FERDIG`, `sist_kjoert` og `antall_feil = 0`. Feil øker `antall_feil`. Feil `synk_id` gir 0 rader.
  - frigi: bare `AKTIV`-rader med egen `synk_id` går til `VENTER`.
  - statistikk: grupperer per status, teller `antall_feil >= 5` og finner eldste `sist_kjoert`.
- Service (`tests/logic/arbeidssoeker_service.rs`): init, steg 2–4, resultat og frigi gjennom `ArbeidssoekerService`.
- Batchløkke (`tests/batch/tiltak_synk_batch.rs`): stopper når claim gir 0 rader, når alle `VENTER` er låst av en annen instans, og når behandleren ber om stopp.
- Metrikker: enhetstester av `SynkStatistikk::fra_rader` (tom tabell, summering og eldste synkronisering).
- Jobb med testcontainers og mockito (`tests/batch/tiltak_synk_batch.rs`): hele kjøringen fra init til `FERDIG`, 401/403/5xx gir `FEILET`, slettet periode under kjøring, og kjøringen stopper ved feil nummer 10 med resten av batchen tilbake i `VENTER`.
- Del aldri en statisk `PgPool` mellom flere `#[tokio::test]`.

## Todos (rekkefølge = commits, hver kan deployes)

1. `domain-mulighetsrommet`: Utført.
2. `lib-mulighetsrommet-client`: Utført.
3. `db-oppsett`: Utført.
4. `periode-konsument`: Utført.
5. `tiltak-synk-repo`: Utført.
6. `synk-jobb`: Utført.
7. `skip-locked-claim`: Utført.
8. `opprydding`: Utført.
9. `metrikker`: Utført.
10. `nais-config`: Utført.
11. (Senere) `kafka-produksjon`: eget design.

## Åpne punkter

- `aktivert = true` under `[synk]` i `config/nais/app_config.toml`. Synk slås av med `false`.
- `isReady` mangler sjekk av database og konsument (se Nais).
- Konsumenttester mangler (se Teststrategi).
- Dette er en betaversjon. Sikkerhetsgjennomgangen er utsatt og gjøres før prod.
