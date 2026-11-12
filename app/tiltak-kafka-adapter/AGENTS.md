# Plan: synkronisering av tiltakshistorikk (tiltak-kafka-adapter)

## Problem og tilnærming

`tiltak-kafka-adapter` er i dag et skjelett med health-endepunkter. Appen skal:

1. Konsumere `paw.arbeidssokerperioder-v1` og holde en arbeidsliste med aktive perioder i Postgres.
2. Kjøre en skedulert jobb hvert 10. minutt som synkroniserer hver arbeidssøker høyst hver 2. time.
3. Hente alle identer fra `paw_kafka_key_gen` og tiltakshistorikk fra Mulighetsrommet.
4. (Senere) Produsere Kafka-melding og oppdatere status i samme logiske operasjon.

Arkitekturen skal tåle flere instanser. Forslaget er å koordinere i Postgres med en betinget `UPDATE ... WHERE synk_status = 'VENTER'` (uten `FOR UPDATE`) og bruke `synk_id` som vakt per kjøring.

## Forslag til arkitektur

Tabellen, topicet og intervallene følger `AGENTS.md`. Valgene nedenfor, inkludert statusverdier og bruk av `synk_id`, er forslag utover kravene der.

| Område | Valg | Begrunnelse |
|---|---|---|
| Skedulering | `tokio::time::interval` med `MissedTickBehavior::Delay` | Samme mønster som `veileder-oppgave`. Ingen ny avhengighet. Kjøringer i samme instans overlapper aldri. |
| Koordinering mellom instanser | Postgres: betinget flipp `VENTER` → `AKTIV` med `RETURNING`, `synk_id` som vakt ved statusoppdatering | Trenger ikke leader election eller distribuert lås. |
| Kilde til arbeidslisten | HWM-konsument (`paw_kafka::hwm`) på periode-topic, Avro via schema registry, `eksterne_hendelser::Periode` | Samme mønster som `kartlegging-api`. |
| Identer | `paw_key_gen_client::finn_identiteter`, send alle `FOLKEREGISTERIDENT` (gjeldende og historiske) | API-et krever historiske identer for komplett historikk. |
| Auth mot Mulighetsrommet og key-gen | Entra M2M via `texas_client` | Avklart med Mulighetsrommet. |
| Ukjente enum-verdier | `Ukjent(String)` | Nye tiltakstyper skal ikke stoppe jobben. |
| Lagring av tiltakshistorikk | Ingen | Data går bare videre i Kafka-meldingen. |

### Tabell `arbeidssoekere`

```sql
CREATE TABLE arbeidssoekere (
    periode_id         UUID PRIMARY KEY,
    identitetsnummer   VARCHAR(20)  NOT NULL,
    periode_startet    TIMESTAMP(6) NOT NULL,
    synk_id            UUID         NOT NULL,
    synk_status        VARCHAR(20)  NOT NULL,
    synk_status_endret TIMESTAMP(6) NOT NULL,
    synk_sist_kjoert   TIMESTAMP(6) NOT NULL,
    synk_antall_feil   INT          NOT NULL
);

CREATE INDEX arbeidssoekere_periode_id_idx ON arbeidssoekere (periode_id);
CREATE INDEX arbeidssoekere_synk_status_idx ON arbeidssoekere (synk_status, synk_status_endret);
```

- Periode startet: Forslaget er `INSERT ... ON CONFLICT (periode_id) DO NOTHING`, med `periode_startet` fra perioden, en ny `synk_id`, status `VENTER`, `synk_status_endret` satt til nå og `synk_antall_feil = 0`. Siden `synk_sist_kjoert` ikke kan være null, settes den ved opprettelse til eldre enn synkroniseringsintervallet slik at perioden kan behandles ved neste kjøring.
- Periode avsluttet: `DELETE` på `periode_id`.

Forslaget bruker UTC konsekvent for tidsfeltene, som er `TIMESTAMP(6)` uten tidssone. `synk_sist_kjoert` settes ved vellykket synkronisering; verdien ved opprettelse er en startverdi, ikke tidspunktet for en faktisk kjøring. Feilede forsøk kan prøves på nytt etter ventetiden under, selv om det er mindre enn to timer siden forsøket.

### Statusmaskin

```
VENTER --flipp--> AKTIV --ok--> FERDIG
                    |--feil---> FEILET
                    |--avbrutt-> AVBRUTT
AKTIV (synk_status_endret eldre enn ett skeduleringsintervall)    --> VENTER
FEILET (synk_status_endret eldre enn økende ventetid, se under)   --> VENTER
FERDIG (synk_sist_kjoert eldre enn synkroniseringsintervall)      --> VENTER
AVBRUTT --> VENTER (ved neste kjøring)
```

### Én kjøring (hvert 10. minutt per instans)

1. Opprett en unik kjørings-ID og sett `feil = 0`. Kjørings-ID-en lagres i `synk_id` når raden tas av kjøringen.
2. **Gjenopprett:**
   - `AKTIV` → `VENTER` når `synk_status_endret` er eldre enn skeduleringsintervallet.
   - `AVBRUTT` → `VENTER` med en gang.
   - `FEILET` → `VENTER` med økende ventetid. Ventetiden dobles for hver feil og stopper ved synkroniseringsintervallet. Med 10 minutters intervall blir den 10, 20, 40 og 80 minutter, og deretter fast 2 timer:
     ```sql
     UPDATE arbeidssoekere
     SET synk_status = 'VENTER', synk_status_endret = timezone('UTC', now())
     WHERE synk_status = 'FEILET'
       AND synk_status_endret < timezone('UTC', now()) - LEAST(
             $1::interval * power(2, LEAST(synk_antall_feil - 1, 10)),
             $2::interval);
     ```
     Ved `FEILET` økes `synk_antall_feil` med 1. Ved `FERDIG` settes `synk_antall_feil = 0`.
3. **Planlegg:** `FERDIG` → `VENTER` når `synk_sist_kjoert` er eldre enn synkroniseringsintervallet.
4. **Løkke:** Flipp opptil `batch_stoerrelse` (500) rader fra `VENTER` til `AKTIV` uten `FOR UPDATE`. Postgres har ikke `UPDATE ... LIMIT`, så grensen ligger i en sub-select:
   ```sql
   UPDATE arbeidssoekere
   SET synk_status = 'AKTIV', synk_status_endret = timezone('UTC', now()), synk_id = $1
   WHERE synk_status = 'VENTER'
     AND synk_sist_kjoert < timezone('UTC', now()) - $3::interval
     AND periode_id IN (
       SELECT periode_id FROM arbeidssoekere
       WHERE synk_status = 'VENTER'
         AND synk_sist_kjoert < timezone('UTC', now()) - $3::interval
       ORDER BY synk_status_endret
       LIMIT $2)
   RETURNING periode_id, identitetsnummer, periode_startet;
   ```
   Den ytre `WHERE synk_status = 'VENTER'` gjør flippen trygg ved samtidighet. Hvis to instanser velger samme rad, blokkerer den andre `UPDATE`-en til den første committer. Postgres sjekker deretter `WHERE` på nytt mot den nye radversjonen (READ COMMITTED), og raden blir ikke med i den samtidige flippen. Den som taper, får færre enn 500 rader.

   Behandle batchen (se under) og oppdater hver rad med vakt:
   ```sql
   UPDATE arbeidssoekere
   SET synk_status = $3, synk_status_endret = timezone('UTC', now()), ...
   WHERE periode_id = $1 AND synk_id = $2 AND synk_status = 'AKTIV';
   ```
   Vakten på `synk_id` hindrer at en treg kjøring overskriver en rad som er satt tilbake til `VENTER` og tatt av en annen kjøring.
5. **Avbryt:** Forslaget er å avbryte kjøringen ved 401 eller 403 fra Mulighetsrommet eller key-gen. Ellers: hvis `feil > maks_feil_per_kjoering` (10), sett resten av radene med denne `synk_id` og status `AKTIV` til `AVBRUTT`, og avslutt kjøringen.
6. **Neste batch:** Fortsett til flippen returnerer 0 rader. Hvis den returnerer 0 fordi en annen instans tok de samme radene, sjekk om det fortsatt finnes `VENTER`-rader med `synk_sist_kjoert` eldre enn synkroniseringsintervallet, og prøv igjen før løkken stopper.

Steg 2 og 3 er idempotente, så det gjør ingenting at flere instanser kjører dem samtidig.

**Lange batcher:** En batch trenger ikke bli ferdig innen 10 minutter. Rader i `AKTIV` blir ikke tatt av andre kjøringer før gjenopprettingen har satt dem tilbake til `VENTER`, etter ett intervall. Skjer det mens den første kjøringen fortsatt jobber, kan arbeidssøkeren behandles to ganger. Vakten avviser en foreldet statusoppdatering, men hindrer ikke doble API-kall eller Kafka-meldinger. Mekanismen oppfyller derfor ikke kravet om å unngå gjentatt prosessering fullt ut; avklar lease-fornyelse eller annen koordinering før implementering.

### Behandling av én batch

1. Hent identer fra key-gen per arbeidssøker (`finn_identiteter`). Bygg et kart `fnr -> periode_id`.
2. Kall Mulighetsrommet med identene i grupper på `identer_per_kall`. Én arbeidssøker har ofte flere identer, så 500 arbeidssøkere gir mer enn 500 identer. Vi starter med én arbeidssøker per kall og prøver 500 etter testing.
3. Grupper `historikk` på `norskIdent` via kartet, slik at hver periode får sin egen liste.
4. (Senere) Send Kafka-melding per arbeidssøker, og sett `FERDIG` med `synk_sist_kjoert` satt til gjeldende UTC-tid.

Feiltelleren teller **feilede kall**, ikke feilede rader. Ett feilet kall med 500 arbeidssøkere setter alle 500 til `FEILET`, men øker telleren med 1.

### Kafka og database (designes senere)

DB og Kafka kan ikke dele en atomisk transaksjon, og outbox er uaktuelt fordi historikken ikke skal lagres. Forslaget er å sende til Kafka og vente på leveringsbekreftelse, og deretter commite status `FERDIG`. Det gir at-least-once, så konsumentene må tåle duplikater. Dette avklares i designet av Kafka-produksjon. Perioden kan bli avsluttet mens raden er `AKTIV`. DB-vakten stopper da statusoppdateringen, men Kafka-meldingen kan alt være sendt. Dette må også håndteres i Kafka-designet. Frem til da kaller synkroniseringen en `TiltakshistorikkPublisher`-trait med en implementasjon som bare logger antall deltakelser.

### Forslag til config (`app_config.toml`)

```toml
skedulering_intervall = "PT10M"
synkronisering_intervall = "PT2H"
batch_stoerrelse = 500
identer_per_kall = 1   # arbeidssøkere per kall mot Mulighetsrommet, justeres etter testing
maks_feil_per_kjoering = 10
```

Kafka-config og otel-directives er kopiert fra `kartlegging-api` og rettes (`group_id_prefix`, `directives`).

### Observerbarhet

- `tiltakshistorikk_synkronisert_total`, `tiltakshistorikk_feilet_total`, `tiltakshistorikk_avbrutt_total` (counters)
- `arbeidssoekere_antall{status}` (gauge)
- `arbeidssoekere_eldste_synkronisering_sekunder` (gauge)
- `arbeidssoekere_vedvarende_feil` (gauge, rader med `synk_antall_feil >= 5`), med varsel i Grafana
- `#[instrument]` på kjøring og batch med `synk_id` og `periode_id`. Ingen fnr i logger eller spans.

### Nais

- `gcp.sqlInstances` (Postgres), `kafka.pool`, `azure.application.enabled: true`
- `accessPolicy.outbound`: Mulighetsrommet tiltakshistorikk og `paw-kafka-key-generator`. Namespace for Mulighetsrommet må verifiseres.
- `isReady` skal vise status for database og konsument.
- `replicas.max: 2` i dev, så oppsettet med flere instanser blir testet.
- Ingen CPU-limits. Mål minnebruken og juster fra 32Mi.
- Scope i dev: `api://dev-gcp.paw.paw-kafka-key-generator/.default` (som i `kartlegging-api`).
- Vurder DB-audit-logging fordi tabellen lagrer fnr.

## 🔴 Rød sone: forstå dette grundig

- SQL for flipp, gjenoppretting, planlegging og statusoppdatering med `synk_id`-vakt (samtidighet mellom instanser)
- Avbruddslogikken (feilteller og `AVBRUTT`)
- Rekkefølgen Kafka-sending og DB-commit (når Kafka designes)

Standard: Fase 4 lager testskjelett og stubber med `TODO` for disse. Utvikleren skriver koden selv, med mindre noe annet blir avtalt.

Grønn sone: domenemodell, klient, migrasjoner, Nais, config og metrikk-boilerplate.

## Teststrategi

- `domain/mulighetsrommet`: serde-tester på eksempel-JSON for alle tre deltakelsestyper og for `Ukjent`.
- `lib/mulighetsrommet_client`: mockito, `token_client_stub`, statuskoder 200/401/403/5xx.
- Konsument: aktiv periode oppretter en rad med `periode_startet`, gjentatt melding bevarer fremdriften, og avsluttet periode sletter raden.
- `arbeidssoekere` med testcontainers-Postgres: første gangs behandling, to samtidige flipper overlapper ikke, gjenoppretting av gamle `AKTIV`-rader, økende ventetid for `FEILET` (1, 2 og over 5 feil), nullstilling av `synk_antall_feil` ved `FERDIG`, vakten avviser oppdatering fra feil `synk_id`, planlegging etter 2 timer, avbrudd ved over 10 feil og avbrudd med en gang ved 401 eller 403.
- Test lange batcher og avklart lease-mekanisme med flere instanser. Test at `synk_sist_kjoert` settes ved vellykket behandling og at feilede forsøk kan gjenopptas.
- Del aldri en statisk `PgPool` mellom flere `#[tokio::test]`.

## Todos (rekkefølge = commits, hver kan deployes)

1. `domain-mulighetsrommet`: crate `domain/mulighetsrommet`, modul `tiltakshistorikk` med request/response og `oneOf` som `#[serde(tag = "type")]`, enum-verdier med `Ukjent(String)`.
2. `lib-mulighetsrommet-client`: crate `lib/mulighetsrommet_client` etter mønsteret i `paw_key_gen_client` (config, error, client med `hent_historikk(identer)`). Mock-crate under `test/` hvis det trengs.
3. `db-oppsett`: `paw_sqlx` i appen, migrasjoner for HWM-tabell og `arbeidssoekere`, config, oppstart i `main.rs`.
4. `periode-konsument`: HWM-konsument som legger inn og sletter rader i arbeidslisten.
5. `arbeidssoekere-repo`: SQL-funksjoner (flipp, gjenopprett, planlegg, oppdater, avbryt) med tester. 🔴
6. `synk-jobb`: skedulert jobb med key-gen, Mulighetsrommet, publisher-trait og feilteller. 🔴 (avbrudd)
7. `metrikker`: counters og gauges.
8. `nais-config`: Postgres, Kafka, Azure, accessPolicy, ryddet config.
9. (Senere) `kafka-produksjon`: eget design.

## Åpne punkter

- `identer_per_kall`: start med 1, test 500. Sjekk responstid og om Mulighetsrommet har grense på antall identer.
- Avklar hvordan en aktiv kjøring beholder eierskapet under lange batcher, uten at en krasjet instans blokkerer videre behandling.
- Avklar om `synk_sist_kjoert` skal bety siste vellykkede synkronisering, som foreslått her, og om kortere retry-intervall for feilede forsøk er ønsket.
- App- og namespace-navn og scope for Mulighetsrommet i dev og prod.
- Dette er en betaversjon. Sikkerhetsgjennomgangen er utsatt og gjøres før prod.
