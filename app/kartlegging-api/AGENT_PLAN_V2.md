# Plan for logikk og datamodell for oversikt over ledighet for arbeidssøkere

> Kombinert og oppdatert versjon av `AGENT_PLAN_REVISED.md` og `AGENT_PLAN_REVISED_2.md`.
> Steg som er verifisert implementert i koden er fjernet.
> Feilaktig antakelse om gap-sjekk på bekreftelse-hendelser er korrigert.

---

## Mål

Gi saksbehandlere oversikt over porteføljen for sitt kontor, med fokus på arbeidssøkeres ledighet og andre viktige
datapunkter. Skal kunne filtrere på kontor og ledighet nyere enn en gitt dato.

---

## Kildedata

### Arbeidssøkerperiode
Kafka-hendelser på `paw.arbeidssokerperioder-v1` topic. Avro-kodet. Én hendelse ved periode start (`avsluttet==null`),
én ved avslutning (`avsluttet!=null`).

- `periode.id` — UUID, unik identifikator for perioden.
- `periode.identitetsnummer` — Identitet til arbeidssøker.
- `periode.startet.tidspunkt` — Tidspunkt for periode start.
- `periode.avsluttet.tidspunkt` — Tidspunkt for avslutning. `null` ved start.

### Bekreftelse
Kafka-hendelser på `paw.arbeidssoker-bekreftelse-v1` topic. Avro-kodet. Sendes inn hver 14. dag.

- `bekreftelse.id` — UUID, unik id for bekreftelsen.
- `bekreftelse.periode_id` — Tilhørende periode.
- `bekreftelse.bekreftelsesloesning` — f.eks. `ARBEIDSSOEKERREGISTERET`, `DAGPENGER`, `FRISKMELDT_TIL_ARBEIDSFORMIDLING`.
- `bekreftelse.svar.gjelder_fra` — Tidspunkt bekreftelsen gjelder fra.
- `bekreftelse.svar.gjelder_til` — Typisk 14 dager etter `gjelder_fra`.
- `bekreftelse.svar.har_jobbet_i_denne_perioden` — Boolean.
- `bekreftelse.svar.vil_fortsette_som_arbeidssoeker` — Boolean. Lagres, men påvirker ikke `arbeidsledig_fra`.

### På-vegne-av
Kafka-hendelser på `paw.arbeidssoker-bekreftelse-paavegneav-v1` topic. Avro-kodet.

### Kontortilknytning
Kafka-hendelser på `poao.siste-oppfolgingsperiode-v3` topic. Tre hendelsestyper (diskriminert via `sisteEndringsType`):

- `OPPFOLGING_STARTET` og `ARBEIDSOPPFOLGINGSKONTOR_ENDRET` — inneholder `kontor.*`, `ident`, `aktor_id`.
- `OPPFOLGING_AVSLUTTET` — inneholder kun `id`. Fjerner raden.

### Identiteter (REST)
REST-kall til kafka-key-generator ved periode start. Gir `arbeidssoeker_id` og gjeldende `folkeregisterident`.

### Persondata (REST)
REST-kall til PDL ved periode start. Gir `fornavn`, `mellomnavn`, `etternavn`.

---

## Databasemodell

- **`arbeidssoekere`** — Én rad per arbeidssøker (`id` = `arbeidssoeker_id` fra key-gen).
- **`kartlegginger`** — Én rad per arbeidssøker-periode. Holder `arbeidsledig_fra`. Fremmednøkler: `arbeidssoeker_id` → `arbeidssoekere.id`, `periode_id` → `perioder.id`.
- **`perioder`** — Én rad per periode. Holder `avsluttet_tidspunkt`.
- **`bekreftelser`** — Alle bekreftelser per periode.
- **`kontortilknytninger`** — Én rad per oppfølgingsperiode. PK er oppfolgingsperiode-UUID. Kobles mot `arbeidssoekere` via `aktor_id`-join (ingen direkte FK).

---

## Forretningslogikk

### Korrekt antakelse: gap-sjekk

**Gap-sjekk gjelder kun ved ny periode-start**, ikke ved bekreftelse-hendelser.

- **Ved ny periode:** Sammenlign `ny_periode.startet_tidspunkt - forrige_periode.avsluttet_tidspunkt`.
  Om gap er ≤ `maks_sammenhengende_periode_gap_dager` (14 dager, inklusiv), viderefør `arbeidsledig_fra` fra forrige kartlegging.
- **Ved bekreftelse:** Ingen gap-sjekk. `arbeidsledig_fra` settes kun basert på `har_jobbet`-flagget og eksisterende verdi.

### 1. Periode-hendelse (implementert)

**A. Periode startet:**
1. Upsert perioden i `perioder`.
2. Hent identiteter fra kafka-key-generator (REST).
3. Finn eller opprett arbeidssøker (PDL-kall for navn kun ved ny arbeidssøker).
4. Finn eller opprett kartlegging for perioden:
   - Ny kartlegging: Beregn `arbeidsledig_fra` fra forrige avsluttede kartlegging (gap-sjekk) eller fra eksisterende bekreftelser.
   - Eksisterende kartlegging: Oppdater `arbeidsledig_fra` fra eksisterende verdi eller bekreftelser.

**B. Periode avsluttet:**
- Upsert perioden (setter `avsluttet_tidspunkt`). Ingen endring i `kartlegginger`.

**Gap-sjekk for ny kartlegging (implementert i `utled_arbeidsledighet_fra_tidligere_kartlegging`):**

| Tilstand | Handling |
|----------|----------|
| Ingen tidligere kartlegging | `arbeidsledig_fra = null` |
| Forrige `arbeidsledig_fra IS NULL` | `arbeidsledig_fra = null` |
| Forrige `arbeidssoeker_til IS NULL` (aktiv periode — uventet) | `arbeidsledig_fra = null` |
| Gap > 14 dager | `arbeidsledig_fra = null` |
| Gap ≤ 14 dager (inklusiv) | Viderefør forrige `arbeidsledig_fra` |

> **Merk:** Grenseverdien er korrekt `<` (eksklusiv) — gap på nøyaktig 14 dager utløser ny ledighetsperiode. Navngiving (`periode_gap_grense_for_ledighet`) uttrykker dette eksplisitt.

### 2. Bekreftelse-hendelse (implementert)

1. Upsert bekreftelsen i `bekreftelser`.
2. Finn kartlegging for `bekreftelse.periode_id`. Ikke funnet → logg advarsel, return Ok.
3. Oppdater `arbeidsledig_fra`:

| Tilstand | Handling |
|----------|----------|
| `har_jobbet=true` | Sett `arbeidsledig_fra = null` |
| `har_jobbet=false` OG `arbeidsledig_fra IS NULL` | Sett `arbeidsledig_fra = bekreftelse.svar.gjelder_fra` |
| `har_jobbet=false` OG `arbeidsledig_fra IS NOT NULL` | Behold eksisterende — ingen endring |

**Ingen gap-sjekk for bekreftelse-hendelser.**

### 3. Bekreftelse-på-vegne-av-hendelse (implementert)

Upsert i `bekreftelse_paa_vegne_av`.

### 4. Opplysninger-hendelse (implementert)

Upsert i `opplysninger`. Ingen endring i `kartlegginger`.

### 5. Profilering-hendelse (implementert)

Upsert i `profileringer`. Slett tilhørende egenvurdering om profilering-id er endret.

### 6. Egenvurdering-hendelse (implementert)

Upsert i `egenvurderinger`.

### 7. Oppfølgingsperiode-hendelse (implementert)

`OPPFOLGING_STARTET` / `ARBEIDSOPPFOLGINGSKONTOR_ENDRET` → insert eller update i `kontortilknytninger`.
`OPPFOLGING_AVSLUTTET` → delete fra `kontortilknytninger` på oppfolgingsperiode-UUID.

---

## Gjenstående oppgaver

### Oppgave 1 — Opprydding i `main.rs`

**Fil:** `src/main.rs`

`clear_db` er fortsatt aktiv med TODO-kommentar:

```rust
// TODO: Fjern før prodsetting!!!
clear_db(&pg_pool).await?;
```

**Hva som skal gjøres:**
1. Fjern `clear_db(&pg_pool).await?` og TODO-kommentaren over.
2. Fjern `clear_db` fra import: `use paw_sqlx::postgres::{clear_db, init_db}` → `use paw_sqlx::postgres::init_db`.

`sqlx::migrate!("./migrations")` er allerede aktiv og korrekt — ingen endring nødvendig der.

---

### Oppgave 2 — Bugfix aktiv-periode-filter i porteføljespørringer

**Fil:** `src/model/dao/arbeidssoeker.rs`

Funksjonene `count_by_kontortilknytning` og `select_by_kontortilknytning` henter arbeidssøkere basert på kontortilknytning, men filtrerer ikke på aktiv arbeidssøkerperiode. Dette medfører at avsluttede perioder kan inkluderes i porteføljeoversikten.

**Nåværende WHERE-klausul (begge funksjoner):**
```sql
WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidsledig_fra NOTNULL AND k.arbeidsledig_fra > $3
```

**Hva som skal gjøres:**
- Legg til `AND k.arbeidssoeker_til IS NULL` for å filtrere på aktive perioder.

Resulterende WHERE-klausul:
```sql
WHERE kt.kontor_id = $1
  AND kt.kontor_type = ANY($2)
  AND k.arbeidsledig_fra NOTNULL
  AND k.arbeidsledig_fra > $3
  AND k.arbeidssoeker_til IS NULL
```

---

### ✅ Thread safety — ingen endringer nødvendig

Analyse av alle variabler som krysser trådgrenser via `tokio::spawn`:

| Variabel | Type | Trådgrense | Status |
|---------|------|-----------|--------|
| `app_config` | `Arc<AppConfig>` | `kafka_consumer_task`, `metrics_task` | ✅ |
| `app_state` | `Arc<AppState>` | `create_kafka_consumer`, `web_server_task` | ✅ |
| `pg_pool` | `PgPool` (internt Arc) | Alle 3 spawntasks | ✅ |
| `key_gen_client` | `Arc<PawKeyGenClient>` | `kafka_consumer_task` via prosessor | ✅ |
| `pdl_client` | `Arc<PDLClient>` | `kafka_consumer_task` via prosessor | ✅ |
| `auth_state` | `Arc<AuthState>` | `web_server_task` via router | ✅ (Arc fra konstruktør) |
| `http_client` | `reqwest::Client` (internt Arc) | Brukes kun under init — ikke sendt til tasks | ✅ |
| `consumer` | `StreamConsumer<HwmRebalanceHandler>` | Moves inn i `kafka_consumer_task` | ✅ |
| `message_processor` | `KartleggingMessageProcessor` | Moves inn i `kafka_consumer_task` | ✅ (alle felter Arc-wrappet) |
| `hwm_version` | `i16` | `kafka_consumer_task` | ✅ (Copy) |

`KartleggingMessageProcessor` er ikke Arc-wrappet, men det er korrekt — den er eid av én task og deles ikke. Alle 7 sub-prosessorer inne i den er `Arc<XxxProcessor>`. `AvroDeserializer` holder `Arc<AvroDecoder<'static>>`.

Rustkompilatoren håndhever `Send + 'static`-kravene ved `tokio::spawn` og ville avvist koden om noe var feil.

---

## Implementeringsrekkefølge (anbefalt)

1. **Oppgave 1** — Enkel opprydding, ingen logikkrisiko.
2. **Oppgave 2** — SQL-bugfix, viktig for riktige data i porteføljeoversikten.

---

## Fremtidige forbedringer (ikke i scope nå)

- Koble `kontortilknytninger` mot `arbeidssoekere` via direkte FK (i dag: JOIN på `aktor_id` i spørringstid).
