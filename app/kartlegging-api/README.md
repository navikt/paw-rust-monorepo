# Kartlegging API

API som kunne kartlegge porteføljen til saksbehandlere for sitt kontor. Det er en aggregering av data på tvers av
Arbeidssøkerregisteret med fokus på arbeidssøkeres ledighet og andre viktige datapunkter. Skal kunne filtrere på
ledighet og egenvurdering nyere enn en gitt dato.

## Utleding av ledighet

* Ikke aktiv arbeidssøker
    * Registrerer seg som arbeidssøker
        * Var tidligere registrert som arbeidssøker
            * Det er mindre en 14 dager siden avslutning
                * Tidligere ledighet er satt -> Viderefør tidligere ledighet
            * Det er over 14 dager siden avslutning -> Ikke sett ledighet (venter på bekreftelse)
* Aktiv arbeidssøker
    * Leverer bekreftelse
        * Bekreftelse er eldre enn periode -> ignorer
        * Bekreftelse er ikke eldre enn periode
            * Har jobbet -> Resett ledighet
            * Har ikke jobbet
                * Ledighet ikke satt
                    * Bekreftelse overlapper periode start -> Sett ledighet til start av periode
                    * Bekreftelse overlapper ikke periode start -> Sett ledighet til start av bekreftelse

OBS! Bekreftelser fra `Dagpenger` og `Friskmeldt til arbeidsformidling` kan ha start (og stopp) fra før perioden ble startet.

### Flytdiagram

#### Arbeidssøker registrerer seg

```mermaid
graph LR
    periode_startet((Periode startet)) --> har_tidligere_periode{Har tidligere periode?}
    har_tidligere_periode -->|Nei| ingen_ledighet[Sett ingen ledighet]
    har_tidligere_periode -->|Ja| er_avsluttet_nylig_ja{Er avsluttet nylig?}
    er_avsluttet_nylig_ja -->|Nei| ingen_ledighet
    er_avsluttet_nylig_ja -->|Ja| viderefør_ledighet[Viderefør ledighet]
    ingen_ledighet --> ferdig((Ferdig))
    viderefør_ledighet --> ferdig
```

#### Arbeidssøker leverer bekreftelse

```mermaid
graph LR
    bekreftelse_levert((Bekreftelse levert)) --> er_bekreftelse_eldre_enn_periode{Er bekreftelse eldre enn periode?}
    er_bekreftelse_eldre_enn_periode -->|Ja| ferdig((Ferdig))
    er_bekreftelse_eldre_enn_periode -->|Nei| bekrefter_har_jobbet{Bekrefter har jobbet?}
    bekrefter_har_jobbet -->|Ja| ingen_ledighet[Sett ingen ledighet]
    bekrefter_har_jobbet -->|Nei| er_ledighet_satt{Er ledighet allerede satt?}
    er_ledighet_satt -->|Ja| ferdig
    er_ledighet_satt -->|Nei| overlapper_bekreftelse_periode_start[Overlapper bekreftelse periode start?]
    overlapper_bekreftelse_periode_start -->|Ja| ledighet_fra_periode_start[Sett ledighet fra periode start]
    overlapper_bekreftelse_periode_start -->|Nei|ledighet_fra_bekreftelse[Sett ledighet fra bekreftelse]
    ingen_ledighet --> ferdig
    ledighet_fra_periode_start --> ferdig
    ledighet_fra_bekreftelse --> ferdig
```

### Tidslinjer

#### Ikke tidligere vært arbeidssøker

```mermaid
gantt
    dateFormat YYYY-MM-DD
    tickInterval 1week
    weekday monday
    Ledighet: done, 2026-01-05, 28d
    Ledighet: done, 2026-02-16, 14d
    Registrering: milestone, 2026-01-05, 1m
    Er arbeidssøker: active, 2026-01-05, 56d
    Levert bekreftelse: milestone, 2026-01-16, 1m
    Har ikke jobbet: active, 2026-01-05, 14d
    Levert bekreftelse: milestone, 2026-01-30, 1m
    Har ikke jobbet: active, 2026-01-19, 14d
    Levert bekreftelse: milestone, 2026-02-13, 1m
    Har jobbet: crit, active, 2026-02-02, 14d
    Levert bekreftelse: milestone, 2026-02-27, 1m
    Har ikke jobbet: active, 2026-02-16, 14d
```

#### Har tidligere vært arbeidssøker siste 14 dager

```mermaid
gantt
    dateFormat YYYY-MM-DD
    tickInterval 1week
    weekday monday
    Ledighet: done, 2026-01-05, 49d
    Avslutning: milestone, 2026-01-19, 1m
    Var arbeidssøker: done, 2026-01-05, 14d
    Levert bekreftelse: milestone, 2026-01-16, 1m
    Har ikke jobbet: done, 2026-01-05, 14d
    Registrering: milestone, 2026-01-26, 1m
    Er arbeidssøker: active, 2026-01-26, 42d
    Levert bekreftelse: milestone, 2026-02-06, 1m
    Har ikke jobbet: active, 2026-01-26, 14d
    Levert bekreftelse: milestone, 2026-02-20, 1m
    Har ikke jobbet: active, 2026-02-09, 14d
    Levert bekreftelse: milestone, 2026-03-06, 1m
    Har jobbet: crit, active, 2026-02-23, 14d
```

## Dataelementer

* Identitet
    * Fra periode start
    * Vedlikehold ved hjelp av identitet-topic
* ArbeidssøkerId
    * Hentet fra Kafka Key Gen ved periode start
* Navn
    * Hent fra PDL ved periode start
* Periode
* Kontortilhørlighet
    * Liste
    * ~~ [DAB datamodell](https://ao-oppfolgingskontor.intern.dev.nav.no/sdl) ~~
    * [DAB oppfolgingsperiode](https://navikt.github.io/veilarboppfolging/veilarboppfolgingV1/#operation-send-siste-oppfolgingsperiode-v3)
* Ledighet
    * Ledig siden dato
        * nulles ved "har jobbet"
        * gjelder fra for første bekreftelser
        * om har svart "har ikke jobbet" med gap på mer enn 14 dager, sett med fra dato etter gap
    * Siste gjelder til
* Bekreftelse
    * Fra dato
    * Til dato
* Profilering -> Slette tidligere egenvurdering om egenvurdering.profilering_id != ny profilering_id?
    * Siste
* Egenvurdering
    * Siste
* På-vegne-av -> Ny på-vegne-av-start så legges kilde til i listen (om den ikke finnes fra før). Tom liste =
  Arbeidssøkerregisteret har ansvaret selv.
    * Liste

### Hendelser

Logikk for mottak av hendelser.

```mermaid
graph LR
    periode_startet((Periode startet)) --> lagre_periode[Lagre periode]
    lagre_periode --> hent_pdl_data[Hent PDL-data]
    hent_pdl_data --> lagre_navn[Lagre navn]
    lagre_navn --> done((Ferdig))
    opplysninger((Opplysninger)) --> ledig_siden_satt{Er ledig_siden satt?}
    ledig_siden_satt -->|Ja| lagre_opplysninger_for_ja[Lagre opplysninger]
    ledig_siden_satt -->|Nei| lagre_opplysninger_for_nei[Lagre opplysninger]
    lagre_opplysninger_for_ja --> done
    lagre_opplysninger_for_nei --> kalkulere_ledig_siden_for_opplysninger[Kalkuler ledig_siden]
    kalkulere_ledig_siden_for_opplysninger --> done
    bekreftelse((Bekreftelse)) --> har_jobbet_siste_periode{Har jobbet siste periode?}
    har_jobbet_siste_periode -->|Ja| lagre_bekreftelse_for_ja[Lagre bekreftelse]
    har_jobbet_siste_periode -->|Nei| lagre_bekreftelse_for_nei[Lagre bekreftelse]
    lagre_bekreftelse_for_ja --> kalkulere_ledig_siden_for_bekreftelse[Kalkuler ledig_siden]
    kalkulere_ledig_siden_for_bekreftelse --> done
    lagre_bekreftelse_for_nei --> done
    profilering((Profilering)) --> har_eldre_egenvurdering{Har eldre egenvurdering?}
    har_eldre_egenvurdering -->|Ja| lagre_profilering_for_ja[Lagre profilering]
    har_eldre_egenvurdering -->|Nei| lagre_profilering_for_nei[Lagre profilering]
    lagre_profilering_for_ja --> slett_egenvurdering[Slett egenvurdering]
    slett_egenvurdering --> done
    lagre_profilering_for_nei --> done
    egenvurdering(("Egenvurdering")) --> lagre_egenvurdering[Lagre egenvurdering]
    lagre_egenvurdering --> done
    bekreftelse_paa_vegne_av(("Bekreftelse på-vegne-av")) --> lagre_bekreftelse_paa_vegne_av[Lagre bekreftelse på-vegne-av]
    lagre_bekreftelse_paa_vegne_av --> done
    tilknyttet_kontor(("Kontortilhørlighet")) --> lagre_tilknyttet_kontor[Lagre kontortilhørlighet]
    lagre_tilknyttet_kontor --> done
```

## Spørringer

* Ledighet nyere/eldre enn
* Egenvurdert nyere enn

## Sikkerhet

* Azure saksbehandler token med ident
* Tilgangsstyring
* Auditlogging? nei, trenger ikke auditlogge for listevisninger

## Testdata

| identitetsnummer | navn                     | ledig_siden | tilknyttet_kontor                                   |
|------------------|--------------------------|-------------|-----------------------------------------------------|
| 24849098329      | SUNN VESTIBYLE           | 2026-04-30  | 4154:ARBEIDSOPPFOLGING, 0617:GEOGRAFISK_TILKNYTNING |
| 16488315440      | SLAKK KAKAOBØNNE         | 2026-06-03  | 0617:ARBEIDSOPPFOLGING, 0617:GEOGRAFISK_TILKNYTNING |
| 09448718961      | GJENNOMSIKTIG RØDFOTSULE | 2026-06-04  | 0617:GEOGRAFISK_TILKNYTNING                         |
| 24488623539      | EKSPLOSIV SKORPION       |             | 3030:GEOGRAFISK_TILKNYTNING                         |
| 02838698800      | TANKEFULL KLISJÉ         |             | 0118:GEOGRAFISK_TILKNYTNING                         |
| 28888896696      | SAKTE NORDAVIND          |             | 0604:GEOGRAFISK_TILKNYTNING                         |

## Poison Pills i dev

* `paw.arbeidssokerperioder-v1`
    * Partition 0
        * hwm 1730
            * fnr: 13497632174, aktor-id: 2078784314389, arbeidssøker-id: 139451 er splittet og ligger med feil state i
              Arbeidssøkerregister
    * Partition 1
        * hwm 438, 452
            * fnr: 02447919705, arbeidssøker-id: 133777 finnes ikke i lengre PDL
