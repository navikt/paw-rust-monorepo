# V2-oppslag for arbeidsledighet per kontor

## Status

Planen er lagret for videre arbeid. Ingen applikasjonskode er endret eller implementert. Omfanget er avklart, og brukeren har bare bedt om å lagre planen, ikke starte implementeringen.

## Mål og avklart omfang

Opprett `POST /api/v2/arbeidsledighet` for kontoroppslag med to SELECT-statements totalt: separat `COUNT` og ett samlet dataoppslag. Behold v1 som sammenligningsgrunnlag og rollback-alternativ.

Brukeren gjør manuelle ytelsesmålinger. Ingen baseline-målinger, benchmarks, måleharness eller EXPLAIN-kjøring inngår i implementeringen. Representativ kontorstørrelse er cirka 20 000 personer og sidestørrelse 500. Oppgitt HTTP-responstid er cirka 900 ms; ønsket mål er under 500 ms. Tallet er en typisk enkelttid uten kjent statistikk. Planen lover ikke at sammenslåing alene når målet.

V2 støtter bare request-typen `TILKNYTTET_KONTOR`. `IDENTITETSNUMMER` skal avvises eksplisitt som en ugyldig request for dette endpointet, ikke sendes videre til v1.

Eksisterende API-, query- og DAO-logikk skal ikke endres. Brukeren tillater bare nødvendige moduldeklarasjoner og router-wiring i eksisterende filer. Ingen nye avhengigheter, migrasjoner, pool-endringer, auth-endringer, Nais-/CI-endringer eller endringer i eksisterende API-dokumentasjon inngår.

## Analyse av dagens løsning

`finn_for_kontortilknytning_query_request` i `app/kartlegging-api/src/logic/query/arbeidsledighet_query.rs` gjør fire sekvensielle SELECT-statements på samme transaksjon:

| Kall | Funksjon | Arbeid |
|---|---|---|
| 1 | `arbeidssoeker::count_by_kontortilknytning` | Teller unike personer med matchende kontor og aktiv kartlegging, med valgfritt datofilter. |
| 2 | `arbeidssoeker::select_by_kontortilknytning` | Velger personer, dedupliserer via `DISTINCT ON`, sorterer og paginerer. |
| 3 | `ledighetsperiode_kompakt::select_by_arbeidssoeker_ids` | Leser aktiv periode på nytt og henter siste egenvurdering, bekreftelse og på-vegne-av. |
| 4 | `kontortilknytning::select_by_aktor_ids` | Henter alle kontortilknytningene for sidens aktør-ID-er. |

Dette er batch-oppslag, ikke N+1. To statements kan spare rundreiser og gjentatt periodeoppslag. En større JOIN kan derimot overføre flere rader med gjentatte person-/periodekolonner. Faktisk ytelsesgevinst avgjøres av brukerens manuelle målinger.

## Foreslått SQL og mapping

### Separat count

Legg en egen count-funksjon i v2-DAO-en. Behold samme filter og `COUNT(DISTINCT a.id)`-semantikk som v1. COUNT skal gi riktig totalantall også når den forespurte siden er tom.

### Samlet dataoppslag

1. Velg kandidater med eksisterende kontor-, kontortype- og periodefilter. Bruk `EXISTS` for kontormatch slik at flere matchende tilknytninger ikke multipliserer kandidatene.
2. Velg én aktiv kartlegging per person med samme `DISTINCT ON` og sortering som v1. Behold periode-ID og periodekolonnene i resultatet.
3. Bruk en paginert `page`-CTE/subquery før enrichment. LIMIT/OFFSET gjelder personer, ikke kontortilknytningsrader.
4. Hent siste egenvurdering og bekreftelse med page-avgrensede `DISTINCT ON`-CTE-er, og på-vegne-av via LEFT JOIN. Dette følger eksisterende SQL-mønster fremfor å innføre et uverifisert LATERAL-oppslag per person.
5. LEFT JOIN alle kontortilknytninger på sidens aktør-ID-er etter pagineringen. Ikke begrens responsens tilknytninger til søkekontoret.
6. Returner typed SQLx-rader med tydelige kolonnealiaser og nullable enrichment-felter. Kontortilknytningens fysiske ID brukes som identitet; to like kontorverdier fra ulike fysiske rader skal ikke slås sammen.
7. Sorter det endelige resultatet med samme person-sorteringsnøkler som siden. Grupper i Rust etter arbeidssøker-ID og bevar første forekomsts rekkefølge. Opprett én kompakt periode per person, ikke én per JOIN-rad.

Ingen JSON-aggregasjon eller ny indeks inngår. Det reduserer implementeringsomfanget og holder konvertering til eksisterende DTO-er i Rust.

### Semantikk som skal bevares

- Aktiv periode betyr `arbeidssoeker_til IS NULL`.
- Datofilter er strengt `arbeidsledig_fra > ledig_siden`; uten filter tillates NULL.
- Periodevalg er `arbeidsledig_fra DESC NULLS LAST`, deretter `arbeidssoeker_fra DESC`.
- Ytre sortering i v1 er `sort_arbeidsledig_fra ASC`, deretter `sort_arbeidssoeker_fra {dir}`. Ikke endre dette til at begge nøkler følger `sort_order`.
- Standard paging er side 1, sidestørrelse 1000 og ASC. Det representative målescenarioet på 500 endrer ikke standarden.
- Siste egenvurdering velges per periode etter `tidspunkt DESC`, siste bekreftelse etter `gjelder_til DESC`.
- Behold UTC-konvertering, nullable felter og `Arbeidssoekerregisteret` som standard ved tom på-vegne-av-liste.
- Behold responsformat, `hit_size` og `total_count` gjennom eksisterende DTO-er.
- Ikke innfør tie-breakere for like sorteringsnøkler. V1 definerer ikke entydig personrekkefølge ved ties eller rekkefølge på kontortilknytninger.

Før ferdigstilling må to eksisterende edge cases håndteres eksplisitt:

- `aktor_id` har ikke UNIQUE-constraint i undersøkt schema. V1 bruker `HashMap::remove`, slik at bare første person med en delt aktør-ID får tilknytningene. Karakteriser dette tilfellet og bevar v1-atferden i v2; ikke endre kardinalitetssemantikken stille.
- Dataoppslaget i v2 får ett statement-snapshot. V1s tre datastatements kan se forskjellige snapshots under READ COMMITTED. Count forblir separat; det loves ikke et felles snapshot mellom count og data. Ingen endring i transaksjonsisolasjon.

## Filer og registrering

Nye filer:

| Fil | Ansvar |
|---|---|
| `app/kartlegging-api/src/api/v2/mod.rs` | Eksporter v2-endpointmodulen. |
| `app/kartlegging-api/src/api/v2/arbeidsledighet.rs` | Route, eksisterende auth-/tracing-middleware, kontorspesifikk parsing/validering, transaksjon og eksplisitt feilrespons. |
| `app/kartlegging-api/src/logic/query/arbeidsledighet_v2_query.rs` | Standardverdier, count/data-kall og mapping/gruppering til eksisterende response-DTO. |
| `app/kartlegging-api/src/model/dao/arbeidsledighet_v2.rs` | Typed resultatrad, separat count og samlet data-SQL. |

Eksisterende filer som bare får registrering:

| Fil | Tillatt endring |
|---|---|
| `app/kartlegging-api/src/api/mod.rs` | `mod v2`, opprett v2-rutene og merge dem med eksisterende router. |
| `app/kartlegging-api/src/logic/query/mod.rs` | Moduldeklarasjon for v2-query. |
| `app/kartlegging-api/src/model/dao/mod.rs` | Moduldeklarasjon for v2-DAO. |

Legg nødvendige tester i de nye filenes testmoduler. Ikke endre eksisterende tester eller mapping-funksjoners synlighet. V2-mapping følger v1s konverteringsregler i egne filer fordi isolasjonen er et uttrykkelig krav; dette gir noe duplisering som må holdes konsistent.

## Implementeringsoppgaver

Avklaringen er ferdig. Alle implementeringsoppgaver gjenstår. Opprett tilsvarende session-todos ved gjenopptakelse; denne filen inneholder avhengighetene uavhengig av den opprinnelige session-databasen.

| ID | Oppgave | Avhengighet |
|---|---|---|
| clarify-scope | Ferdig: separat COUNT, bare kontoroppslag i v2, to statements, manuelle målinger og kun registreringsendringer i eksisterende filer. | Ingen |
| characterize-results | Utform korrekthetstestene i nye v2-testmoduler som sammenligner med eksisterende v1-funksjon på samme stabile datasett. Testene ferdigstilles og kjøres når v2 er kompilert og registrert. | clarify-scope |
| implement-v2-dao | Opprett isolert DAO med separat count, page-first SQL og typed JOIN-resultat. | characterize-results |
| implement-v2-query | Opprett query og mapping med uendret response-semantikk. | implement-v2-dao |
| register-v2-route | Opprett kontorspesifikt v2-endpoint og registrer nye moduler/ruter med minimale eksisterende filendringer. | implement-v2-query |
| validate-v2 | Kjør korrekthetstester, build, clippy og formatkontroll; kontroller endringsomfang. Ingen ytelsesmålinger. | register-v2-route |

## Korrekthetsvalidering

Bruk eksisterende Postgres-testhelper og SQLx-migrasjoner, syntetiske testdata og en egen pool/schema per test. Dette er korrekthetstester, ikke ytelsesfixtures med 20 000 personer.

Sammenlign v1- og v2-respons for:

- standard paging og eksplisitt paging, ASC/DESC og NULL-datoer;
- dato på filtergrensen og over grensen, avsluttede perioder og flere aktive perioder;
- flere matchende kontortilknytninger, andre kontorer og fysisk forskjellige tilknytninger med like kontorverdier;
- siste egenvurdering/bekreftelse per periode, manglende enrichment og på-vegne-av-standard;
- delt aktør-ID, delvis siste side, ingen treff og offset utenfor resultatet;
- `hit_size`, `total_count` og alle kompakte person-/periodefelter.

Normaliser bare uspesifisert rekkefølge. Ties over en sidegrense har ikke deterministisk medlemskap i v1; test gyldige medlemmer og kardinalitet fremfor å kreve identiske vilkårlige tie-utvalg.

Test også registrering av `/api/v2/arbeidsledighet`, avvisning av `IDENTITETSNUMMER` og ugyldig paging. Behold auth og eksisterende feilresponsmønster. Ingen request-body eller persondata skal logges.

Kjør `rtk cargo build -p kartlegging-api`, målrettede `rtk cargo nextest run -p kartlegging-api`-selektorer for v2-testene, `rtk cargo clippy -p kartlegging-api -- -D warnings` og `rtk cargo fmt --all -- --check`. Kontroller at eksisterende filendringer er begrenset til de tre registreringsfilene. Ikke kjør formattering som skriver om andre eksisterende filer.

## Manuell ytelsesvurdering

Brukeren måler v1 mot v2 med samme request, cirka 20 000 personer per kontor og sidestørrelse 500. Implementeringen rapporterer ingen målt forbedring og hevder ikke at målet under 500 ms er nådd.

Hvis brukerens målinger viser manglende gevinst, vurder query plans, indekser eller en annen resultatstruktur i en separat oppgave. Dette inngår ikke automatisk i planen.

## Sikkerhet og rollback

🔴 Rød sone: request-validering, periodevalg og paginering må forstås grundig. V2 gjenbruker eksisterende validering og auth uten nye sikkerhetsregler.

V2 bruker samme OAuth2-middleware og ProblemDetails-mønster som v1. Parameterbind alle request-verdier; bare den lukkede `SortOrder`-typen kan brukes til SQL-retning. Valider feil request-type eksplisitt. Ikke logg tokens, fødselsnumre, navn eller SQL-bindverdier.

Det er ingen skjemaendring eller datamigrasjon. V1 forblir tilgjengelig, slik at klienten kan bruke v1 igjen uten å reversere databaseendringer.

## Evidens og begrensninger

Undersøkt: eksisterende arbeidsledighet-handler og query, person-/periode-/kontor-DAO-er og mapping, request-/response-DTO-er, schema-/indeksmigrasjoner, testhelper, modulregistrering og hovedrouter.

Indekser på join-kolonner og siste enrichment finnes i migrasjonene. Den aktive kartleggingsindeksen har DESC uten eksplisitt NULLS LAST, mens spørringen bruker NULLS LAST; faktisk sorteringskostnad er ikke undersøkt. Ingen query plans, live database, effektive pool-innstillinger eller nettverkslatens er undersøkt.

SQL bruker eksplisitte kolonner og personpaginering før enrichment. Ingen JSONB eller nye migrasjoner inngår. Kallene forblir sekvensielle på samme transaksjon og pool-konfigurasjonen endres ikke.

Vurdering: færre statements er gjennomførbart. Ytelsesgevinst og måloppnåelse er ikke dokumentert og overlates til brukerens manuelle målinger.
