# Avvist til oppgave

Applikasjon som oppretter oppgaver i [Oppgave API](https://github.com/navikt/oppgave) ([Swagger](https://oppgave.intern.dev.nav.no/)) basert på avviste hendelser fra arbeidssøkerregisteret.

Når en person under 18 år forsøker å registrere seg som arbeidssøker, blir registreringen avvist. Denne appen sørger for at avvisningen fører til en oppgave som en saksbehandler kan følge opp.

## Flyt

### 1. Mottak av avviste hendelser

Appen konsumerer hendelser fra Kafka-topicen `paw.arbeidssoker-hendelseslogg-v1`. Når en avvist-hendelse med opplysningen `ER_UNDER_18_AAR` mottas, lagres den som en intern oppgave i databasen med status `UBEHANDLET`.

Noen regler gjelder:

- **Kun bruker-initierte avvisninger** — dersom en veileder registrerer en bruker under 18 år, vil det også generere en avvist hendelse, men denne ignoreres. Kun avvisninger trigget av brukeren selv fører til oppgave.
- **Duplikathåndtering** — dersom brukeren prøver å registrere seg flere ganger, opprettes det ikke en ny oppgave. I stedet logges hendelsen i `oppgave_hendelse_logg`.
- **Vannskille** — hendelser eldre enn `OPPRETT_OPPGAVER_FRA_TIDSPUNKT` får status `IGNORERT`. Dette brukes for å koordinere overgang fra legacy-appen ([aia-backend](https://github.com/navikt/aia-backend)) til denne appen.

### 2. Opprettelse av oppgaver i Oppgave API

En bakgrunnsjobb (`opprett_oppgave_task`) kjører med konfigurerbart intervall og oppretter oppgaver mot det eksterne Oppgave API-et:

1. Henter de eldste ubehandlede oppgavene fra databasen (batch)
2. Shuffler listen for å unngå at flere pods alltid tar samme oppgave
3. Bruker optimistisk CAS-lock (Compare-And-Swap) via `UPDATE ... WHERE status = $expected` for å sikre at kun én pod behandler hver oppgave
4. Kaller Oppgave API for å opprette oppgaven eksternt
5. Oppdaterer status til `OPPRETTET` og logger resultatet i `oppgave_hendelse_logg`

Dersom kallet feiler, settes oppgaven tilbake til `UBEHANDLET` slik at den plukkes opp igjen ved neste kjøring.

### 3. Ferdigstilling av oppgaver

Appen konsumerer hendelser fra Kafka-topicen `oppgavehandtering.oppgave-hendelse-v1`. Når en oppgave ferdigstilles i det eksterne Oppgave API-et, mottar vi en `OPPGAVE_FERDIGSTILT`-hendelse. Appen matcher denne mot vår interne oppgave via `ekstern_oppgave_id`, og oppdaterer status til `FERDIGBEHANDLET`.

## Oppgavestatuser

| Status | Beskrivelse |
|---|---|
| `UBEHANDLET` | Avvist hendelse mottatt, venter på å bli sendt til Oppgave API |
| `OPPRETTET` | Oppgave opprettet i Oppgave API |
| `FERDIGBEHANDLET` | Oppgave ferdigstilt eksternt |
| `IGNORERT` | Hendelse eldre enn vannskillet |

## Database

PostgreSQL med to tabeller:

- **`oppgaver`** — Intern oppgave med kobling til arbeidssøker og ekstern oppgave-id
- **`oppgave_hendelse_logg`** — Hendelseslogg per oppgave for sporing av statusendringer

## Autentisering

Appen bruker [Texas](https://doc.nais.io/auth/) (Token Exchange as a Service) for maskin-til-maskin-token mot Oppgave API via Azure/Entra ID.


## Kjøre lokalt

Appen krever Kafka og PostgreSQL. Bruk docker-compose-filene i `docker/`:

```sh
docker compose -f docker/postgres/docker-compose.yaml up -d
docker compose -f docker/kafka/docker-compose.yaml up -d
cargo run -p avvist-til-oppgave
```

Alternativt kan du trykke på den grønne play-knappen ved `main`-funksjonen i IDE-en.

## Konfigurasjon

Konfigurasjon leses fra TOML-filer under `config/`. I NAIS-miljø brukes filer fra `config/nais/`, lokalt fra `config/local/`. Miljøvariabler substitueres via `serde_env_field`.

| Konfigurasjon | Beskrivelse |
|---|---|
| `topic_hendelseslogg` | Kafka-topic for hendelsesloggen |
| `topic_oppgavehendelse` | Kafka-topic for oppgavehendelser |
| `opprett_oppgaver_task_interval_minutes` | Intervall for oppgaveopprettelses-tasken |
| `opprett_oppgaver_task_batch_size` | Antall oppgaver per batch |
| `opprett_oppgaver_fra_tidspunkt` | Vannskille — hendelser eldre enn dette ignoreres |

## Deploy

Deployes til NAIS via GitHub Actions. Se `nais/nais-dev.yaml` for NAIS-konfigurasjon.
