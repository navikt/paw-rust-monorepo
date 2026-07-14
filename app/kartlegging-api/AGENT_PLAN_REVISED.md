# Revidert plan for logikk og datamodell for oversikt over ledighet for arbeidssøkere

> Dette er en revidert versjon av AGENT_PLAN.md, produsert etter analyse av eksisterende kode og datamodell.
> Avvik og presiseringer er markert med 🔧.

---

## Mål

Gi saksbehandlere oversikt over porteføljen for sitt kontor, med fokus på arbeidssøkeres ledighet og andre viktige
datapunkter. Skal kunne filtrere på kontor og ledighet nyere enn en gitt dato.

---

## Kildedata

### Arbeidssøkerperiode (periode)
Kafka-hendelser på `paw.arbeidssokerperioder-v1` topic. Avro-kodet. Én hendelse ved periode start (`avsluttet==null`),
én ved avslutning (`avsluttet!=null`).

- `periode.id` (periode_id) — UUID, unik identifikator for perioden.
- `periode.identitetsnummer` — Identitet til arbeidssøker.
- `periode.startet.tidspunkt` — Tidspunkt for periode start.
- `periode.avsluttet.tidspunkt` — Tidspunkt for avslutning. `null` ved start.

### Bekreftelse (bekreftelse)
Kafka-hendelser på `paw.arbeidssoker-bekreftelse-v1` topic. Avro-kodet. Sendes inn hver 14. dag.

- `bekreftelse.id` — UUID, unik id for bekreftelsen.
- `bekreftelse.periode_id` — Tilhørende periode.
- `bekreftelse.bekreftelsesloesning` — f.eks. `ARBEIDSSOEKERREGISTERET`, `DAGPENGER`, `FRISKMELDT_TIL_ARBEIDSFORMIDLING`.
- `bekreftelse.svar.gjelder_fra` — Tidspunkt bekreftelsen gjelder fra. Første bekreftelse har `gjelder_fra == periode.startet.tidspunkt`.
- `bekreftelse.svar.gjelder_til` — Typisk 14 dager etter `gjelder_fra`.
- `bekreftelse.svar.har_jobbet_i_denne_perioden` — Boolean.
- `bekreftelse.svar.vil_fortsette_som_arbeidssoeker` — Boolean. Lagres, men påvirker ikke `arbeidsledig_siden`.

### På-vegne-av (bekreftelse_paavegneav)
Kafka-hendelser på `paw.arbeidssoker-bekreftelse-paavegneav-v1` topic. Avro-kodet.

- `paa_vegne_av.periode_id` — Tilhørende periode.
- `paa_vegne_av.bekreftelsesloesning` — Løsning som tar over bekreftelsesansvaret.

### Kontortilknytning (oppfolgingsperiode) 🔧

Kafka-hendelser på `poao.siste-oppfolgingsperiode-v3` topic. Tre hendelsestyper (diskriminert via `sisteEndringsType`):

- `OPPFOLGING_STARTET` — Inneholder `kontor.kontorId`, `kontor.kontorNavn` (type: `ARBEIDSOPPFOLGING`), og `ident`.
- `ARBEIDSOPPFOLGINGSKONTOR_ENDRET` — Inneholder nytt kontor (type: `ARBEIDSOPPFOLGING`), og `ident`.
- `OPPFOLGING_AVSLUTTET` — Inneholder `sluttTidspunkt` og `ident`. Ingen kontor-data.

> **Oppdatering 🔧:** `OppfolgingsperiodeEndret` og `OppfolgingsperiodeAvsluttet` i
> `dab_oppfolgingperioder`-domenet har fått nye felt: `ident: String` (identitetsnummer),
> `aktor_id: String`, `oppfolgingsperiode_id: Uuid` og `start_tidspunkt: DateTime<Utc>`.
> `ident` **brukes fra payloaden** for å slå opp arbeidssoeker — meldingsnøkkelen skal ikke leses.
> `ident` behandles som rå streng (11 siffer, ingen validering).
> `aktor_id` lagres ikke i DB nå — fremtidig forbedring for datakryssreferanse.

> **Merknad:** Alle kontortilknytninger fra dette topicet hardkodes til `ARBEIDSOPPFOLGING` — med overlegg.
> `GEOGRAFISK_TILKNYTNING` er ikke en kilde her per nå, men arkitekturen legger til rette for at
> denne typen kan innhentes fra et annet topic i fremtiden.

### Identiteter (REST)
REST-kall til kafka-key-generator ved periode start.

- `response.arbeidssoeker_id` — Unik BIGINT identifikator for arbeidssøker.
- `response.identiteter` — Liste med alle identiteter. Gjeldende: `identitet.gjeldende==true`.

### Persondata (REST)
REST-kall til PDL ved periode start.

- `response.fornavn`
- `response.mellomnavn` (nullable)
- `response.etternavn`

---

## Databasemodell

Eksisterende tabeller (ingen endringer i skjema nødvendig):

- **`arbeidssoekere`** — Én rad per arbeidssøker (`arbeidssoeker_id` fra key-gen).
- **`kartlegginger`** — Én rad per arbeidssøker-periode. Koblingstabellen mellom arbeidssoeker og
  periode. Holder `arbeidsledig_siden`.
- **`perioder`** — Én rad per periode (upsert ved begge hendelser: start og avslutning).
- **`bekreftelser`** — Alle bekreftelser per periode.
- **`opplysninger`** — Jobbsituasjon-opplysninger per periode.
- **`profileringer`** — Profileringer per periode.
- **`egenvurderinger`** — Egenvurderinger per periode.
- **`bekreftelse_paa_vegne_av`** — Én rad per periode, liste med løsninger.
- **`kontortilknytninger`** — Kontorer per arbeidssoeker (parent_id → arbeidssoekere.id).

---

## Fakta

1. `bekreftelse.svar.gjelder_fra` for **første** bekreftelse er lik `periode.startet.tidspunkt`.
2. `har_jobbet_i_denne_perioden=false` betyr personen **ikke** har jobbet mellom `gjelder_fra` og `gjelder_til`.
3. En kartlegging er alltid knyttet til én og bare én periode via `periode_id`.
4. En arbeidssoeker kan ha **flere** kartlegginger — én per historisk og aktiv periode.
5. `arbeidsledig_siden` i `kartlegginger` representerer starten på **inneværende sammenhengende
   ledighetstrekning** for den gjeldende perioden.

---

## Forretningslogikk

### Definisjoner

- **"Aktiv kartlegging"** for et gitt `periode_id`: den ene kartleggingsraden med `kartlegginger.periode_id = periode_id`.
- **"Forrige kartlegging"** ved ny periode start: kartleggingen med det seneste
  `perioder.avsluttet_tidspunkt IS NOT NULL`, funnet via JOIN mellom `kartlegginger` og `perioder`
  på `parent_id = arbeidssoeker.id`. 🔧 (Krever JOIN med `perioder`-tabellen, ikke direkte felt i
  `kartlegginger`.)

---

### 1. Ved periode-hendelse (`paw.arbeidssokerperioder-v1`)

**Felles for alle periode-hendelser:**
- Upsert perioden i `perioder`-tabellen (insert om ny, update `avsluttet_tidspunkt` om den finnes).

**A. Periode startet (`avsluttet==null`):**

1. Hent identiteter-response via REST til kafka-key-generator.
2. Finn gjeldende identitetsnummer og `arbeidssoeker_id` fra svaret.
3. **Opprett eller finn arbeidssoeker:**
   - Om ingen arbeidssoeker finnes for `arbeidssoeker_id`: hent navn fra PDL, opprett ny rad.
   - Om arbeidssoeker finnes: bruk eksisterende `parent_id`.
4. **Opprett kartlegging for denne perioden — idempotent (skip om raden allerede finnes):** 🔧
   - `kartlegginger.periode_id = periode.id`
   - `kartlegginger.arbeidssoeker_siden = periode.startet.tidspunkt`
   - `kartlegginger.arbeidsledig_siden` = se logikk under.
5. **Bestem `arbeidsledig_siden` for den nye kartleggingen:**
   - Om **ingen eksisterende kartlegginger** finnes for denne `arbeidssoeker_id`:
     → `arbeidsledig_siden = null`
   - Om **eksisterende kartlegginger** finnes — finn "forrige kartlegging":
     - `forrige = kartlegging med seneste avsluttede periode (avsluttet_tidspunkt IS NOT NULL)`
     - Om `forrige` ikke finnes (alle perioder fortsatt aktive — uventet tilstand):
       → `arbeidsledig_siden = null`
     - Om `forrige.arbeidsledig_siden IS NULL`:
       → `arbeidsledig_siden = null`
     - Om `forrige.arbeidsledig_siden IS NOT NULL` OG
       `ny_periode.startet_tidspunkt - forrige_periode.avsluttet_tidspunkt > 14 dager`:
       → `arbeidsledig_siden = null`
     - Om `forrige.arbeidsledig_siden IS NOT NULL` OG
       `ny_periode.startet_tidspunkt - forrige_periode.avsluttet_tidspunkt <= 14 dager`: 🔧
       → `arbeidsledig_siden = forrige.arbeidsledig_siden` (viderefør ledighetsdatoen)

> **Presisering grenseverdi 🔧:** Bruk `<=` (inklusiv) for «innen 14 dager»-betingelsen, slik at
> et gap på nøyaktig 14 dager regnes som sammenhengende.

**B. Periode avsluttet (`avsluttet!=null`):** 🔧

- Upsert perioden (setter `avsluttet_tidspunkt`).
- Ingen endringer i `kartlegginger`. Kartleggingen for denne perioden beholdes intakt med
  eksisterende `arbeidsledig_siden`.

---

### 2. Ved bekreftelse-hendelse (`paw.arbeidssoker-bekreftelse-v1`)

1. Upsert bekreftelsen i `bekreftelser`-tabellen.
2. Finn kartlegging for `bekreftelse.periode_id`:
   - **Om ingen kartlegging finnes for `periode_id`:** 🔧
     - Slå opp identitetsnummer via `perioder.identitetsnummer WHERE id = bekreftelse.periode_id`.
     - Om perioden ikke finnes i DB (ekte out-of-order): logg advarsel og ignorer (kan ikke opprette
       kartlegging uten arbeidssoeker-kontekst).
     - Om perioden finnes: finn eller opprett arbeidssoeker (samme logikk som ved periode start, men
       uten PDL-kall siden navn allerede kan finnes — bruk eksisterende om den finnes, ellers
       skip/logg advarsel siden PDL-kall krever periode-context med key-gen).
     - Opprett ny kartlegging med `arbeidsledig_siden` basert på `har_jobbet`-verdien (se under).
   - **Om kartlegging finnes:** gå videre.
3. **Oppdater `arbeidsledig_siden` basert på bekreftelsesinnhold:**

   | Tilstand | Handling |
   |----------|----------|
   | `har_jobbet=false` OG `arbeidsledig_siden IS NULL` | Sett `arbeidsledig_siden = bekreftelse.svar.gjelder_fra` |
   | `har_jobbet=false` OG `arbeidsledig_siden IS NOT NULL` OG gap ≤ 14 dager | Ikke gjør noe — behold eksisterende dato |
   | `har_jobbet=false` OG `arbeidsledig_siden IS NOT NULL` OG gap > 14 dager 🔧 | Sett `arbeidsledig_siden = bekreftelse.svar.gjelder_fra` |
   | `har_jobbet=true` | Sett `arbeidsledig_siden = null` |

   > **Ny regel 🔧 (manglet i opprinnelig plan):** Gap mellom bekreftelser beregnes som
   > `ny_bekreftelse.gjelder_fra - forrige_bekreftelse.gjelder_til` for gjeldende periode.
   > Om gapet er **> 14 dager** selv med `har_jobbet=false`, betyr det at vi har en periode uten
   > bekreftelse. `arbeidsledig_siden` skal da reset til `ny_bekreftelse.gjelder_fra`, fordi vi ikke
   > vet om personen var ledig i gapet.
   >
   > For å beregne gapet: hent `MAX(gjelder_til) FROM bekreftelser WHERE periode_id = ?` og sammenlign
   > med `ny_bekreftelse.gjelder_fra` **før** du lagrer den nye bekreftelsen.

---

### 3. Ved bekreftelse-på-vegne-av-hendelse (`paw.arbeidssoker-bekreftelse-paavegneav-v1`)

- Upsert i `bekreftelse_paa_vegne_av`: legg til `bekreftelsesloesning` i listen for `periode_id` om
  den ikke allerede finnes. Ny løsning som starter = legg til; eksisterende løsning = no-op.

---

### 4. Ved opplysninger-hendelse (`paw.opplysninger-om-arbeidssoeker-v1`)

- Upsert opplysningene i `opplysninger`-tabellen.
- **Ingen endring i `kartlegginger.arbeidsledig_siden` per nå.** README antyder fremtidig logikk
  for å beregne `ledig_siden` fra opplysninger (f.eks. ved spesifikke jobbsituasjons-verdier), men
  dette er ikke spesifisert og utenfor scope.

---

### 5. Ved profilering-hendelse (`paw.arbeidssoker-profilering-v1`)

- Upsert profileringen i `profileringer`-tabellen.
- Om det finnes en egenvurdering for gjeldende periode med `egenvurdering.profilering_id != ny_profilering.id`:
  slett den gamle egenvurderingen (profileringen har endret seg, egenvurderingen er foreldet).

---

### 6. Ved egenvurdering-hendelse (`paw.egenvurdering-v1`)

- Upsert egenvurderingen i `egenvurderinger`-tabellen.

---

### 7. Ved oppfolgingsperiode-hendelse (`poao.siste-oppfolgingsperiode-v3`) 🔧

> **Ny seksjon — manglet i opprinnelig plan.** Topicet var kommentert ut i `main.rs`.

Prosesseres av `OppfolgingsperiodeProcessor`. Identitetsnummer hentes fra **`ident`-feltet i payloaden**
(ikke fra meldingsnøkkelen). `ident` behandles som rå streng — ingen validering.

**A. `OPPFOLGING_STARTET`:**
1. Les `ident` fra `data.ident` (felt på `OppfolgingsperiodeEndret`).
2. Finn arbeidssoeker via `select_by_identitetsnummer(tx, &ident)`.
3. Om ingen arbeidssoeker finnes: logg advarsel (uten PII), ignorer.
4. Om arbeidssoeker finnes:
   - Kall `delete_by_parent_id_and_type(tx, parent_id, "ARBEIDSOPPFOLGING")`.
   - Sett inn ny `KontortilknytningRow` med `kontor_id`, `kontor_navn`, `kontor_type="ARBEIDSOPPFOLGING"`.

**B. `ARBEIDSOPPFOLGINGSKONTOR_ENDRET`:**
- Samme logikk som `OPPFOLGING_STARTET`.

**C. `OPPFOLGING_AVSLUTTET`:**
1. Les `ident` fra `data.ident` (felt på `OppfolgingsperiodeAvsluttet`).
2. Finn arbeidssoeker via `select_by_identitetsnummer(tx, &ident)`.
3. Om ingen arbeidssoeker finnes: ignorer.
4. Kall `delete_by_parent_id_and_type(tx, parent_id, "ARBEIDSOPPFOLGING")`.
- Begrunnelse: personen er ikke lenger under oppfølging — kontor-tilknytningen er ikke lenger gyldig.

---

## Idempotens og hendelsesrekkefølge 🔧

- Alle operasjoner skal tåle dupliserte Kafka-meldinger (idempotent upsert).
- `kartlegginger` upsert: INSERT om `(parent_id, periode_id)` ikke finnes, ellers no-op (ikke
  overskriv eksisterende `arbeidsledig_siden` blindt).
- `kontortilknytninger` upsert: slett eksisterende av samme type, sett inn ny.
- Out-of-order bekreftelse uten kjent periode: logg og ignorer.

---

## Feilkilder og avklaringspunkter

| # | Spørsmål | Beslutning |
|---|----------|------------|
| 1 | Hvilken kilde gir `GEOGRAFISK_TILKNYTNING`-type kontortilknytninger? | Alle kontortilknytninger fra `poao.siste-oppfolgingsperiode-v3` hardkodes til `ARBEIDSOPPFOLGING` — med overlegg. `GEOGRAFISK_TILKNYTNING` kan komme fra et annet topic i fremtidige utvidelser. |
| 2 | Skal `vil_fortsette=false` påvirke `arbeidsledig_siden` eller noe annet? | Nei — lagres kun, ingen sideeffekt. |
| 3 | Skal `OPPFOLGING_AVSLUTTET` fjerne `ARBEIDSOPPFOLGING`-kontortilknytning? | Ja — se punkt 7C over. |
| 4 | Hva skjer om PDL-kall feiler ved periode start? | La hele transaksjonen feile (retry via Kafka-offset). |
| 5 | Kan en arbeidssoeker ha to aktive perioder samtidig? | Antatt nei i praksis, men systemet skal ikke krasje om det skjer. |
| 6 | Skal `ident` fra `Oppfolgingsperiode` valideres (11 siffer)? | Nei — behandles som rå streng. |
| 7 | Skal `ident` leses fra meldingsnøkkel eller payload? | **Fra payload** (`data.ident`-felt). Meldingsnøkkelen brukes ikke. |
| 8 | Skal `aktor_id` lagres i databasen? | Ikke nå — fremtidig forbedring for datakryssreferanse. |
| 9 | Tester i `dab_oppfolgingperioder` feiler (`missing field 'oppfolgingsperiodeId'`). Hva gjøres? | Utvikler oppdaterer test-JSON med realistiske verdier — ikke en del av denne planen. |
| 10 | `select_by_kontortilknytning` og `count_by_kontortilknytning` filtrerer ikke på aktive perioder. Inkluderes fix? | **Ja** — uten fix viser porteføljeoversikten feil data. |
| 11 | `clear_db()` er aktivt i `main.rs` med TODO-kommentar. | Fjernes som del av dette arbeidet — den sletter all data ved oppstart. |

---

## Manglende DAO-funksjoner 🔧

Følgende funksjoner mangler og må legges til før prosessorene kan implementeres:

### `src/model/dao/kartlegging.rs`

- `count_by_periode_id(tx, periode_id) -> i64`
  ```sql
  SELECT COUNT(*) FROM kartlegginger WHERE periode_id = $1
  ```
- `insert(tx, parent_id, periode_id, startet_tidspunkt, arbeidsledig_siden) -> u64`
  ```sql
  INSERT INTO kartlegginger (parent_id, periode_id, arbeidssoeker_siden, arbeidsledig_siden)
  VALUES ($1, $2, $3, $4)
  ```
- `update_arbeidsledig_siden(tx, periode_id, arbeidsledig_siden: Option<DateTime<Utc>>) -> u64`
  ```sql
  UPDATE kartlegginger SET arbeidsledig_siden = $2 WHERE periode_id = $1
  ```
- `select_by_periode_id(tx, periode_id) -> Option<KartleggingSimpleRow>`
  — ny struct `KartleggingSimpleRow { parent_id: i64, periode_id: Uuid, arbeidsledig_siden: Option<DateTime<Utc>> }`
  ```sql
  SELECT parent_id, periode_id, arbeidsledig_siden FROM kartlegginger WHERE periode_id = $1
  ```
- `select_forrige_avsluttet_by_parent_id(tx, parent_id) -> Option<ForrigeKartleggingRow>`
  — ny struct `ForrigeKartleggingRow { arbeidsledig_siden: Option<DateTime<Utc>>, avsluttet_tidspunkt: DateTime<Utc> }`
  ```sql
  SELECT k.arbeidsledig_siden, p.avsluttet_tidspunkt
  FROM kartlegginger k
  JOIN perioder p ON p.id = k.periode_id
  WHERE k.parent_id = $1 AND p.avsluttet_tidspunkt IS NOT NULL
  ORDER BY p.avsluttet_tidspunkt DESC
  LIMIT 1
  ```

### `src/model/dao/bekreftelse.rs`

- `select_max_gjelder_til_by_periode_id(tx, periode_id) -> Option<DateTime<Utc>>`
  — **må kalles FØR den nye bekreftelsen settes inn**
  ```sql
  SELECT MAX(gjelder_til) FROM bekreftelser WHERE periode_id = $1
  ```

### `src/model/dao/kontortilknytning.rs`

- `delete_by_parent_id_and_type(tx, parent_id, kontor_type: &str) -> u64`
  ```sql
  DELETE FROM kontortilknytninger WHERE parent_id = $1 AND kontor_type = $2
  ```

---

## Implementeringsrekkefølge (forslag)

1. **Nye DAO-funksjoner** (se seksjon over) — ingen avhengigheter på ny kode.

2. **`kartlegging_mutation`-modul** (`src/logic/mutation/kartlegging_mutation.rs`) — ny fil:
   - `opprett_for_periode(tx, parent_id, periode_id, startet_tidspunkt, arbeidsledig_siden)`
     — idempotent: sjekk `count_by_periode_id`, skip om raden allerede finnes.
   - `oppdater_arbeidsledig_siden(tx, periode_id, arbeidsledig_siden)`
   - Registrer i `src/logic/mutation/mod.rs`.

3. **Bugfix aktiv-periode-filter** (`src/model/dao/arbeidssoeker.rs`):
   - `select_by_kontortilknytning_asc`, `select_by_kontortilknytning_desc` og
     `count_by_kontortilknytning` mangler alle filter på aktiv periode.
   - Legg til `LEFT JOIN perioder p ON p.id = k.periode_id` og `AND p.avsluttet_tidspunkt IS NULL`
     i alle tre funksjonene. 🔧

4. **`periode_process.rs`** — fullfør logikk etter at `parent_id` er hentet:
   - Hent `select_forrige_avsluttet_by_parent_id(tx, parent_id)`.
   - Beregn `arbeidsledig_siden` (se logikk i seksjon 1 over, inklusiv `<=`-grenseverdi).
   - Kall `kartlegging_mutation::opprett_for_periode(...)`.
   - Fjern alle `.unwrap()` — bruk `?` eller `ProcessorError`. Ingen PII i logger.

5. **`bekreftelse_process.rs`** — fullfør logikk etter at bekreftelse er lagret:
   - Hent `select_max_gjelder_til_by_periode_id` **før** `lagre_hendelse`-kallet.
   - Finn kartlegging: `select_by_periode_id(tx, periode_id)`.
   - Ikke funnet: logg advarsel (uten PII), ignorer.
   - Oppdater `arbeidsledig_siden` per tabellen i seksjon 2 over (gap-regler inkludert).

6. **`oppfolgingsperiode_process.rs`** — implementer kontortilknytning-logikk:
   - Les `ident` fra `data.ident` (payload, ikke meldingsnøkkel) — se seksjon 7 over.
   - Bruk `select_by_identitetsnummer(tx, &ident, 0, 1, Descending)` for oppslag.
   - Dispatch på `Startet`/`Endret`/`Avsluttet` — se seksjon 7 over.

7. **`main.rs`**:
   - Uncomment `SISTE_OPPFOLGINGSPERIODE_V3_TOPIC` i topics-listen.
   - Fjern `clear_db(&pg_pool).await?` og tilhørende TODO-kommentar.

---

## Fremtidige forbedringer (ikke i scope nå)

- Legg til `aktor_id: String`-kolonne i `arbeidssoekere`-tabellen (Flyway-migrasjon + DAO)
  for sammenstilling med andre systemer.
