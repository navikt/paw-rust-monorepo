# Periode-før-bekreftelse rekkefølgeproblem ved kaldstart

## Bakgrunn

`kartlegging-api` abonnerer på 7 topics med én `StreamConsumer` (se `src/kafka/bootstrap.rs`,
`src/kafka/consumer.rs`, `src/kafka/topics.rs`). rdkafka gir ingen garanti for rekkefølge på
tvers av topics/partisjoner — kun innad i én partisjon. Dette er identisk med ren
`KafkaConsumer.poll()` i Java; det er kun Kafka Streams (via internt per-partisjon buffer,
"prosesser laveste timestamp"-strategi og `max.task.idle.ms`) som gir den typen synkronisering,
og det finnes ingen tilsvarende, vedlikeholdt løsning i Rust/rdkafka-økosystemet.

## Funn: avhengighetsgraf

Kodegjennomgang viser at **kun** `PAW_BEKREFTELSE_TOPIC` faktisk er avhengig av at
`PAW_PERIODE_TOPIC` er prosessert først:

- `src/logic/process/bekreftelse_process.rs` slår opp `kartlegging`-raden via `periode_id`.
  Hvis perioden ikke finnes ennå, logges kun en `warn!("Fant ingen kartlegginger for
  periode-id")` og oppdateringen av `arbeidsledig_fra`/`arbeidssoeker_til` går varig tapt —
  det finnes ingen retry-mekanisme.
- `src/logic/process/bekreftelse_paavegneav_process.rs` er **ikke** avhengig av periode —
  den har egen tabell og gjør insert/update uten periode-oppslag (håndterer "stopp uten
  eksisterende rad" med kun en warning, ikke tap av periode-relatert data).
- `opplysninger_process.rs`, `profilering_process.rs`, `egenvurdering_process.rs` gjør
  uavhengige insert/update uten periode-join.
- `oppfolgingsperiode_process.rs` er uavhengig (annen nøkkel, `oppfolgingsperiode_id`).

Avhengighetsgrafen er altså smal: **periode → bekreftelse**. Problemet oppstår kun ved
kaldstart/backfill fra offset 0, siden bekreftelse i normal drift kommer dager etter periode —
lenge etter at perioden allerede er konsumert.

## Viktig tilleggsfunn: periode og bekreftelse er co-partisjonert

`periode` og `bekreftelse` produseres med samme nøkkel-strategi, slik at korrelerte meldinger
(samme `periode_id`) alltid havner i **samme partisjonsnummer** i begge topics (forutsatt lik
partisjonstall og standard partisjoner/assignor). Dette er en vesentlig forenkling sammenlignet
med det generelle "synkroniser alle 7 topics på tvers av alle partisjoner"-problemet:

- Vi trenger aldri koordinere på tvers av partisjoner — kun partisjon N i periode må være foran
  partisjon N i bekreftelse, for hver N uavhengig.
- Med standard `range`-assignor (og likt partisjonstall) vil partisjon N av periode og partisjon
  N av bekreftelse normalt tildeles **samme forbruker-pod** i konsumentgruppen — akkurat den
  egenskapen Kafka Streams selv er avhengig av for co-partisjonerte joins. Dette bør verifiseres
  eksplisitt for kartlegging-api sin partisjonstall/assignor-konfigurasjon.

Dette åpner for enklere/mer presise løsninger enn en global watermark-basert catch-up, se
sammenligning under.

## Sammenligning av tre alternativer

### Alternativ 1: Engangs snapshot-catch-up (opprinnelig forslag)

Hent high watermark for periode ved oppstart, drain periode (evt. kun de partisjonene som
trenger det) til watermarken via en midlertidig konsument, og start deretter hovedkonsumenten.

**Design (per partisjon, ved bruk av co-partisjonering)**:
1. Snapshot high watermark per periode-partisjon ved oppstart (`fetch_metadata`/
   `fetch_watermarks`, samme mønster som `bootstrap.rs::bootstrap_missing_hwms`).
2. Sammenlign lagret HWM (Postgres, `get_hwm`) mot snapshotet per partisjon — hopp over
   partisjoner som allerede er innhentet (no-op i normal drift).
3. For partisjoner som trenger catch-up: midlertidig konsument med eget group.id-suffiks,
   `assign()` periode-partisjonen fra lagret HWM-offset, drain til snapshotet via eksisterende
   `hwm_process_message`/`PeriodeProcessor`-pipeline.
4. Start hovedkonsument (`create_kafka_consumer` + `subscribe(TOPICS)`) som i dag.
5. Flere pods: Postgres advisory lock rundt catch-up per partisjon.
6. Sikkerhetsventil: maks-tid/deadline med logging/metric ved overskridelse.

**Vurdering**:
- ✅ Lav løpende kostnad — kjører kun én gang, ingen endring i steady-state hot path.
- ✅ Gjenbruker eksisterende mønster fra `bootstrap.rs` (kjent, testet tilnærming i kodebasen).
- ⚠️ Løser **kun** kaldstart-scenarioet. Hvis periode skulle bli forsinket av andre grunner i
  fremtiden (produsent-replay, feilretting, migrering), er det ingen beskyttelse — gaten fjernes
  permanent etter første oppstart.
- ⚠️ Krever separat midlertidig konsument/group-id + advisory lock — ekstra bevegelige deler.
- ⚠️ Watermarken er et øyeblikksbilde; må tåle at nye periode-meldinger kommer inn *under*
  draining (løses av at hovedkonsumenten tar over rett etter, men øker kompleksitet i
  overgangen).

### Alternativ 2: Reaktiv per-partisjon pause/resume basert på faktisk mangel (anbefalt)

I stedet for å forhåndsberegne en watermark, la hovedkonsumenten selv oppdage og reagere på
manglende periode, og utnytt co-partisjoneringen til å vite nøyaktig hvilken partisjon som må
vente.

**Design**:
1. Behold én `StreamConsumer` abonnert på alle 7 topics (ingen separat konsument/group-id).
2. Når en bekreftelse-melding prosesseres og tilhørende periode **ikke** finnes ennå
   (samme sjekk som allerede gjøres i `bekreftelse_process.rs`), i stedet for å kun logge en
   warning: `consumer.pause()` denne ene topic-partisjonen, `seek()` tilbake til meldingens
   offset (slik at den leveres på nytt senere), og noter hvilken `periode_id` denne partisjonen
   venter på.
3. Fortsett konsumentloopen som normalt — andre partisjoner/topics er upåvirket, siden pause
   kun gjelder den spesifikke bekreftelse-partisjonen.
4. Hver gang en periode-melding prosesseres, sjekk om noen pauset bekreftelse-partisjon ventet
   på nettopp denne `periode_id`-en (eller enklere: bare prøv å resume alle pausede
   bekreftelse-partisjoner og la dem selv oppdage om periode nå finnes) — `resume()` når
   perioden er tilgjengelig.
5. Sikkerhetsventil: hvis en partisjon forblir pauset lenger enn en terskel (f.eks. periode
   aldri kommer — reelt datakvalitetsproblem), logg/varsle og evt. gi opp ventingen (fall
   tilbake til dagens warning-oppførsel for akkurat den meldingen) slik at én manglende periode
   ikke blokkerer partisjonen for alltid.

**Vurdering**:
- ✅ Mest robust: løser **både** kaldstart og enhver fremtidig forbigående out-of-order-hendelse
  (produsent-retry, replay, migrering) — ikke bare et engangstilfelle.
- ✅ Ingen separat konsument, group-id eller advisory lock nødvendig — alt skjer i den
  eksisterende `StreamConsumer`/`HwmRebalanceHandler`-konteksten, som allerede garanterer at
  denne poden eier begge partisjon-N-ene (gitt co-partisjonering + standard assignor).
  Enklere feilmodell enn alternativ 1.
- ✅ Ingen forhåndsberegnet watermark — presist og selvjusterende per nøkkel/partisjon.
- ⚠️ Krever pause/seek/resume-logikk i konsumentløkken (moderat kompleksitet, men lite
  kodeareal — kun `kafka/consumer.rs`).
- ⚠️ Forutsetter at co-partisjonering + partisjonstall/assignor faktisk gir samme pod eierskap
  til periode-N og bekreftelse-N — bør verifiseres (se åpne spørsmål).

### Alternativ 3: Lagre alt rått, avstem med en periodisk bakgrunnsjobb

Fjern all cross-topic-avhengighet fra Kafka-konsumeringen: lagre periode- og
bekreftelse-hendelser rått (upsert per event-id) uten noen join ved mottak. En periodisk
bakgrunnsjobb (samme mønster som `app/veileder-oppgave/src/opprett_ekstern_oppgave_task.rs`,
som allerede bruker `tokio::time::interval` + en "ubehandlet"-status-kolonne) skanner
periodisk etter bekreftelser der periode nå finnes, og gjør da join + oppdaterer
`kartlegginger`, markert som "behandlet".

**Vurdering**:
- ✅ Mest generelle/robuste korrekthetsmodell — helt uavhengig av konsumeringsrekkefølge,
  fungerer uansett hvor lenge en periode er forsinket.
- ✅ Konsument-koden blir enklere (ren insert, ingen join-logikk eller pause/resume i hot path).
- ✅ Kjent mønster i monorepoet (`opprett_ekstern_oppgave_task.rs`), lite ny "oppfinnelse".
- ⚠️ Størst endring: krever nye rå-tabeller/migrasjoner, endret skrivemodell (insert-rått →
  periodisk join i stedet for direkte oppdatering), og en ny bakgrunnsprosess å drifte/overvåke.
- ⚠️ Introduserer **latens** på bekreftelse-drevne `kartlegginger`-oppdateringer (bundet av
  jobb-intervallet, typisk minutter) — er det akseptabelt for API-konsumenter av
  kartleggingsdata som forventer nær-sanntid?
- ⚠️ Mer kode totalt sett når man regner med rå-tabeller, statusfelt, opprydding av gamle
  rå-rader, og selve jobben — større blastradius enn alternativ 1/2 selv om hver enkelt del er
  enkel.

## Anbefaling

**Alternativ 2** (reaktiv per-partisjon pause/resume) gir best balanse mellom robusthet og
innsats, gitt co-partisjoneringen:
- Løser problemet mer generelt enn alternativ 1 (ikke bare kaldstart) uten ekstra
  infrastruktur (ingen midlertidig konsument, group-id eller advisory lock).
- Vesentlig mindre endring enn alternativ 3 (ingen skjemaendring, ingen ny bakgrunnsjobb, ingen
  latens-regresjon).
- Bygger på et mønster (pause/seek/resume per topic-partisjon) som er godt støttet av
  `rdkafka` og lett å isolere til `kafka/consumer.rs`.

Alternativ 1 beholdes som et enklere, men snevrere, fallback-alternativ dersom pause/seek/resume
per partisjon viser seg vanskeligere å få robust enn antatt. Alternativ 3 vurderes kun dersom
avhengighetsgrafen senere vokser til å omfatte flere topic-par eller mer komplekse
join-scenarioer, der en generell avstemmingsjobb blir mer lønnsom enn punktvise fikser.

### Ikke i scope / bevisst utelatt

- `bekreftelse_paavegneav`, `opplysninger`, `profilering`, `egenvurdering`,
  `oppfolgingsperiode` — ingen periode-avhengighet funnet, trenger ikke gating.
- Ekte "foreldreløs" bekreftelse (periode finnes aldri) er et datakvalitetsproblem, ikke et
  rekkefølgeproblem — løses ikke av denne endringen. **Oppdatert etter senere risikoanalyse (se
  "Implementert løsning" under):** det finnes ikke lenger noen tidsbasert gi-opp-vei for dette;
  partisjonen forblir pauset til en operatør griper inn manuelt, og en langvarig pause oppdages
  i stedet via helsesjekk-varsling. `test_process_bekreftelse_3_uten_periode` tester derfor i dag
  kun at meldingen returnerer `AwaitingDependency` uten kobling, ikke et gi-opp-fallback.

## Foreslåtte implementasjonssteg for Alternativ 2 (for senere gjennomføring)

1. Bekrefte avhengighetsomfang (periode → bekreftelse) — evt. med domeneeier.
2. Verifisere co-partisjonering i praksis: bekrefte at periode og bekreftelse har likt
   partisjonstall og at standard assignor faktisk kolokerer partisjon N av begge topics på
   samme pod (viktig forutsetning for at alternativ 2 er korrekt).
3. Implementere pause/seek-logikk i `kafka/consumer.rs`: når bekreftelse-prosessering
   oppdager manglende periode, pause den aktuelle topic-partisjonen og seek tilbake til
   meldingens offset.
4. Implementere resume-logikk: trigges når en periode-melding prosesseres (evt. enklere:
   forsøksvis resume av alle pausede bekreftelse-partisjoner ved hvert periode-gjennombrudd,
   og la den vanlige "finnes periode nå"-sjekken avgjøre om meldingen kan prosesseres).
5. Legge til sikkerhetsventil: maks-tid en partisjon kan forbli pauset før logging/varsling
   og evt. fallback til eksisterende warning-oppførsel for den spesifikke meldingen. **Reversert
   senere** — en tidsbasert gi-opp/fallback ble vurdert for risikabelt gitt kaldstart-skala
   (~1 million periode-hendelser per partisjon) og er erstattet med helsesjekk-varsling uten
   fallback, se "Implementert løsning" under.
6. Legge til observability: logg/metric for pause/resume-hendelser og gjeldende pause-varighet
   per partisjon.
7. Enhetstester for pause/seek/resume-beslutningslogikken, isolert fra faktisk Kafka-klient.
8. Integrasjonstest som simulerer kaldstart med bekreftelse-melding for en periode plassert
   før perioden er konsumert, og verifiserer at kartlegging-raden til slutt reflekterer
   bekreftelsen korrekt etter at periode er prosessert og partisjonen resumeres. Eksisterende
   test for reelt foreldreløse bekreftelser (`test_process_bekreftelse_3_uten_periode`) skal
   fortsatt bestå uendret.

## Åpne spørsmål / risiko

- Er co-partisjonering (likt partisjonstall + standard assignor ⇒ samme pod eier periode-N og
  bekreftelse-N) faktisk garantert i dagens oppsett, eller kun en antagelse? Må verifiseres før
  alternativ 2 kan anses korrekt.
- Hvor lang pause-tid er akseptabel/forventet i kaldstart-scenarioet (påvirker valg av
  sikkerhetsventil-terskel og om readiness-probe bør reflektere "cataching up")?
- Hvis alternativ 1 likevel foretrekkes (f.eks. fordi co-partisjonering ikke kan verifiseres
  eller pause/seek/resume viser seg vanskeligere i praksis enn antatt), må advisory
  lock-strategien sjekkes mot annen bruk av Postgres advisory locks i appen.

## Implementert løsning (gjeldende tilstand)

Alternativ 2 er implementert, med DB-backet pause-tilstand (se begrunnelse under) og uten en
tidsbasert gi-opp-mekanisme (fjernet igjen etter en senere risikoanalyse, se eget avsnitt).
Faktiske filer/moduler (avviker noe fra de opprinnelige forslagsnavnene
`pause_controller.rs`/`hwm_pause_store/` — navngivningen ble justert underveis):

- `src/kafka/hwm/hwm_dao.rs` — `HwmStatus`-enum (`Active`/`Paused`) og DAO-funksjoner
  (`get_by_topic_partition`, `find_by_status`, `update_as_paused`, `update_as_active`) mot samme
  `hwm`-tabell som brukes av delt `paw_rdkafka_hwm`, utvidet med `status`/`status_timestamp`.
- `src/kafka/hwm/hwm_pause.rs` — `pause_and_seek`, `resume_all`, `paused_partitions`,
  `mark_resolved`. Rene `async fn` (ingen `futures::executor::block_on`-bro lenger — fjernet i en
  senere opprydding siden funksjonene allerede var async).
- `src/kafka/hwm/hwm_pause_processor.rs` — `hwm_process_paused_partitions`, kalt fra
  `kafka/consumer.rs` etter hver melding; avgjør pause/resume basert på prosesseringsresultatet og
  `app_config.kafka.synced_topics()`. Ren beslutningsfunksjon `should_attempt_resume` er
  enhetstestet isolert fra `rdkafka`.
- `src/kafka/hwm/hwm_pause_task.rs` — bakgrunnstask (se eget avsnitt under, "Fjernet
  sikkerhetsventil/gi-opp — erstattet med helsesjekk-varsling").
- `src/model/error.rs` — `PayloadProcessorError::AwaitingDependency` + `awaiting_dependency`/
  `map_awaiting_dependency`, som opprinnelig planlagt: `bekreftelse_process.rs` returnerer denne i
  stedet for å committe, `internal_hwm_process_message` i `paw_rdkafka_hwm` ruller da tilbake hele
  transaksjonen (inkludert HWM) uten at noen endring i det delte biblioteket var nødvendig.

**Co-partisjonering**: løst som opprinnelig planlagt — opt-in-felt
`partition_assignment_strategy: Option<String>` på delt `KafkaConfig` (`lib/paw_rdkafka`), satt
til `"range"` for kartlegging-api (`config/{local,nais}/kafka_config.toml`).

**Rebalansering**: løst uten å røre `HwmRebalanceHandler`, som opprinnelig planlagt — stale
`PAUSED`-rader for partisjoner poden ikke lenger eier er ufarlige, siden `resume_all()` og
pause-sjekken alltid filtrerer DB-rader mot `consumer.assignment()` før noe `rdkafka`-kall gjøres.

**DB-backet pause-tilstand** (i stedet for opprinnelig `Mutex<HashMap<(String, i32), Instant>>` i
en tidlig iterasjon): `hwm`-tabellen er utvidet (samme migrasjon som oppretter tabellen —
`migrations/20260529010000_hwm_table.sql` — redigert direkte siden migrasjonen ikke var kjørt i
dev/prod ved endringstidspunktet; **ikke** en ny migrasjonsfil) med en `status`-kolonne
(`ACTIVE`/`PAUSED`, styrt av Rust-enumet `HwmStatus` — bevisst **ikke** en Postgres-side
`CHECK`-constraint, for å unngå duplisert vedlikehold ved fremtidige enum-utvidelser) og
`status_timestamp`. Motivasjon: overlever pod-restart/rebalansering (pause-varighet er korrekt
selv om poden som satte pausen har krasjet), fjerner `Mutex`/panikkrisiko fra hot path, og gjør
pause-tilstand observerbar med ren SQL uavhengig av Prometheus-metrikkene.

**Skille «prøv igjen» fra «bekreftet løst»**: `resume_all()` gjør **kun** `consumer.resume()` mot
`rdkafka`-klienten (ingen DB-skriving) — kalles optimistisk ved enhver vellykket melding på et
synkronisert topic, uten å vite om et gitt ventende bekreftelse-forsøk faktisk lykkes. En egen,
separat operasjon, `mark_resolved`, kalles fra `hwm_process_paused_partitions` sin
suksess-gren for den spesifikke topic/partition-en til meldingen som nettopp lyktes — det er
eneste sted DB-status faktisk flyttes til `ACTIVE`. `update_as_paused` er guardet
(`WHERE status <> 'PAUSED'`) slik at gjentatte pause-forsøk ikke friskner opp
`status_timestamp` for en partisjon som allerede er markert pauset.

### Fjernet: gi-opp-mekanisme — erstattet med helsesjekk-varsling ved fastlåst partisjon

Den opprinnelige planen (og en tidlig implementasjon) inneholdt en tidsbasert "gi opp"-mekanisme:
etter en terskel (`PARTITION_PAUSE_TIMEOUT`, 10 minutter) falt `bekreftelse_process.rs` tilbake
til å lagre bekreftelsen uten kobling til `kartlegginger` (samme sluttresultat som
`test_process_bekreftelse_3_uten_periode` alltid har testet). Denne mekanismen er **fjernet
helt** etter en risikoanalyse:

- Periode og bekreftelse er co-partisjonert og periode-topic har uendelig retention, og periode
  produseres alltid før korrelert bekreftelse — det er derfor i praksis umulig for en bekreftelse
  å mangle sin periode permanent når hele periode-partisjonen er konsumert.
- I prod har periode-topic ~1 million hendelser per partisjon (6 partisjoner) ved kaldstart —
  en fast 10-minutters terskel var trolig allerede feilkalibrert for denne skalaen, og risikerte å
  gi opp (permanent, stille tap av periode↔bekreftelse-kobling) under helt normal drift, ikke bare
  ved reelle datakvalitetsproblemer.
- Konsekvensen av å fjerne gi-opp helt er at en partisjon i prinsippet kan forbli pauset for
  alltid dersom periode-før-bekreftelse-invarianten likevel skulle brytes (produsentfeil,
  GDPR-sletting/tombstoning, endret partisjonstall/co-partisjonering, migrerings-/replay-verktøy).

I stedet for gi-opp finnes nå en operativ varslingsmekanisme i `src/kafka/hwm/hwm_pause_task.rs`:
en bakgrunnstask (`hwm_pause_timeout_task`) sjekker periodisk (konfigurerbart intervall, se under)
om noen pauset partisjon har stått pauset lenger enn en konfigurerbar terskel
(`stuck_partition_threshold`, default 1 time — vesentlig høyere enn den gamle
10-minutters-konstanten, som utelukkende var kalibrert for et helt annet formål). Hvis terskelen
er nådd: logger `tracing::error!`, inkrementerer metrikken
`paw_kafka_partition_stuck_events_total`, og kaller `app_state.set_is_alive(false)`
(`health_and_monitoring::simple_app_state::AppState`) — samme mønster som allerede brukes i delt
`HwmRebalanceHandler` for uopprettelige rebalanseringsfeil.

**Viktig driftskonsekvens**: `/internal/isAlive` er koblet til Nais/Kubernetes sin
**liveness**-probe (ikke readiness). Siden pause-tilstanden er DB-persistert og overlever
pod-restart, vil en *reelt* fastlåst partisjon (ikke bare en lang, men normal kaldstart) føre til
at poden restartes gjentatte ganger til noen griper inn manuelt — restarten løser ikke selve
årsaken. Dette er bevisst valgt som et høylytt, synlig signal fremfor stille datatap, men bør
suppleres med et alert på `paw_kafka_partition_stuck_events_total` og på sikt et manuelt
"unpause"-verktøy/runbook (finnes ikke i dag).

Den periodiske, ubetingede `resume_all()`-en som tidligere kjørte i denne bakgrunnstasken (for å
støtte gi-opp-tidsuret) er også fjernet — vurdert som overflødig, siden den reaktive
resume-veien (`hwm_process_paused_partitions`, trigget for hver vellykkede melding på et
synkronisert topic) allerede dekker normal gjenoppretting.

### Konfigurerbare terskler

`STUCK_PARTITION_THRESHOLD` og sjekkintervallet var opprinnelig hardkodede Rust-konstanter.
Begge er nå flyttet til `AppConfig` (`src/config.rs`, ny `HwmPauseConfig`-struct) og satt i
`config/{local,nais}/app_config.toml` under `[hwm_pause]`:

```toml
[hwm_pause]
check_interval = "PT30S"
stuck_partition_threshold = "PT1H"
```

Samme ISO8601-`Duration`-mønster som `metrics_task_interval` (`duration::iso8601::deserialize`).
Dette gjør at terskelen kan justeres per miljø uten kodeendring/redeploy av selve logikken.

### Observability

Prometheus-metrikker i `logic/metrics/kafka_metrics.rs`:

- `paw_kafka_partition_paused` (gauge) og `paw_kafka_partition_pause_events_total`/
  `paw_kafka_partition_resume_events_total` (counters) — pause/resume-hendelser.
- `paw_kafka_partition_pause_duration_seconds` (histogram) — hvor lenge en partisjon sto pauset
  før resume.
- `paw_kafka_partition_stuck_events_total` (counter) — ny metrikk, erstatter de tidligere
  `paw_kafka_partition_safety_valve_events_total` og `paw_bekreftelse_gir_opp_total` (begge
  fjernet sammen med sikkerhetsventil-/gi-opp-mekanismene de målte).

### Testdekning (gjeldende)

- 7 rene enhetstester uten ekte Kafka-klient eller DB:
  - `hwm_pause_processor.rs`: 3 tester av `should_attempt_resume`
    (synced-topics-filtrering).
  - `hwm_pause_task.rs`: 4 tester av `is_any_partition_stuck` (ingen pauset, ingen over terskel,
    én over terskel, nøyaktig på terskelen).
- Integrasjonstest `test_process_bekreftelse_4_kaldstart_periode_ankommer_senere` i
  `bekreftelse_process.rs`: simulerer hele kaldstart-forløpet (bekreftelse ankommer først →
  `AwaitingDependency`, periode prosesseres, bekreftelsen re-leveres og kobles korrekt til
  `kartlegginger`-raden).
- `test_process_bekreftelse_3_uten_periode` er oppdatert: forventer nå ubetinget
  `Err(AwaitingDependency)` (ingen gi-opp-vei lenger å falle tilbake på; kommentarer/asserts som
  nevnte "gi-opp-terskelen" er ryddet opp).
- Testcontainer-baserte tester (Postgres via `testcontainers`) kunne ikke kjøres til grønt i
  utviklingsmiljøet dette arbeidet ble utført i (miljøspesifikk `Permission denied` mot Docker),
  uavhengig av selve endringen — verifisert ved kompilering og manuell gjennomgang.

### Gjenstående/uverifiserte risikoer

- **Ekstern partisjonstall-verifisering**: partisjonstall for `PAW_PERIODE_TOPIC` og
  `PAW_BEKREFTELSE_TOPIC` i faktiske miljøer (dev-gcp/prod-gcp) er fortsatt ikke verifisert fra
  dette repoet.
- **Nais-manifester**: `replicas` i `nais/nais-{dev,prod}.yaml` viser fortsatt `min=1,max=1`, som
  ifølge bruker er utdatert/misvisende — bevisst utenfor scope, bør rettes separat.
- **Manuelt unpause-verktøy/runbook mangler**: siden gi-opp er fjernet og restart ikke løser en
  reelt fastlåst partisjon, gjenstår behovet for en administrativ måte å sette `status='ACTIVE'`
  manuelt på en `hwm`-rad, samt et Grafana-alert på `paw_kafka_partition_stuck_events_total` — bør
  på plass før denne løsningen er produksjonsklar.
- Docker/testcontainer-basert kjøring av integrasjonstestene feiler fortsatt i dette
  utviklingsmiljøet, uavhengig av denne endringen.

