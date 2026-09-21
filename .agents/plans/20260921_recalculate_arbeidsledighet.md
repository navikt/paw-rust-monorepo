# Plan for lokal korrigering av arbeidsledighet

## Problem og avgrensning

Endringene i `app/kartlegging-api` håndterer bekreftelser som gjelder før
`kartlegging.arbeidssoeker_fra`. Den nye felles utledningen har riktig skille
mellom før, overlapp og etter periodestart, men `BekreftelseProcessor` har en
tidlig guard som hopper over hele utledningen når bekreftelsen slutter før
perioden. Dermed er ikke den lokale flyten og den delte utledningen konsekvente.

Planen dekker bare lokal logikk og tester. Den endrer ikke Flyway-migrasjonen,
HWM-replay eller Kafka-konfigurasjon.

## Avklarte regler

- En bekreftelse med `gjelder_til <= arbeidssoeker_fra` er før perioden og skal
  ignoreres.
- En bekreftelse som overlapper periodestart har
  `gjelder_fra < arbeidssoeker_fra < gjelder_til` og skal gi
  `arbeidsledig_fra = arbeidssoeker_fra` når den ellers etablerer ledighet.
- En bekreftelse som starter på eller etter periodestart skal ved første
  relevante «ikke jobbet»-svar gi `arbeidsledig_fra = gjelder_fra`.
- En tidligere kartlegging med satt `arbeidsledig_fra`, men uten
  `arbeidssoeker_til`, bryter en datainvariant. Koden skal fortsatt feile hardt
  og stanse konsumeringen.

## Foreslått gjennomføring

1. Endre grensesjekken i
   `src/logic/process/kartlegging_process.rs` slik at likhet behandles som før
   perioden. Bruk `gjelder_til <= periode_startet` før overlappsgrenen.
2. Behold den tidlige guarden i
   `src/logic/process/bekreftelse_process.rs` for bekreftelser før perioden.
   Den skal logge og ikke endre `arbeidsledig_fra`. Dette er bevisst og betyr
   at tidligere-kartlegging-fallbacken bare brukes når en behandlet bekreftelse
   ikke selv gir ledighet.
3. Gjør guardens regel identisk med den delte utledningen:
   `gjelder_til <= arbeidssoeker_fra`. Da behandles tidspunktlikhet likt i
   begge kodeveier.
4. Behold `LogicError::precondition_failed` når den nyeste tidligere
   kartleggingen har `arbeidsledig_fra = Some(_)` og fortsatt er åpen. Ikke
   erstatt den med `None`, siden dette skal synliggjøre brudd på
   datainvarianten.
5. Utvid rene enhetstester i `kartlegging_process.rs`:
   - helt før periodestart, inkludert `gjelder_til == arbeidssoeker_fra`
   - overlapp med strikt intervall
   - start lik periodestart
   - etter periodestart
   - reset etter «har jobbet»
6. Utvid integrasjonstester i `bekreftelse_process.rs` gjennom
   `process_payload`:
   - bekreftelse før perioden endrer ikke en eksisterende
     `arbeidsledig_fra`
   - bekreftelse med sluttid lik periodestart endrer ikke verdien
   - en gyldig etterfølgende bekreftelse kan fortsatt oppdatere verdien
   - invariantbruddet fra tidligere åpen kartlegging returnerer feil fra den
     virkelige prosesseringsflyten
7. Kjør målrettede tester for `kartlegging-api`. Testcontainer-testene krever
   Docker-tilgang. Hvis miljøet fortsatt nekter dette, kjør de rene
   enhetstestene og dokumenter den eksterne begrensningen.

## Berørte filer

| Fil | Endring |
|---|---|
| `app/kartlegging-api/src/logic/process/kartlegging_process.rs` | Presiser likhetsgrensen og utvid enhetstester. |
| `app/kartlegging-api/src/logic/process/bekreftelse_process.rs` | Synkroniser guard med grenseverdien og test ekte meldingsflyt. |

## Risiko og kontrollpunkter

- Ikke fjern hard feil for åpen tidligere kartlegging. Den beskytter en
  nødvendig datainvariant.
- Ikke la en før-periode-bekreftelse kjøre fallback eller overskrive
  `arbeidsledig_fra`.
- Kontroller at begge kodeveier bruker samme regel ved
  `gjelder_til == arbeidssoeker_fra`.
- Ikke endre migrasjonen eller replay-strategien som del av denne oppgaven.

## Rød sone

🔴 Rød sone: utledning av `arbeidsledig_fra` og håndtering av brudd på
datainvarianten. Gå nøye gjennom hver test mot forretningsreglene før merge.
