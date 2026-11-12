# Tiltak Kafka Adapter
Applikasjon som jevnlig henter informasjon om tiltakshistorikk for arbeidssøkere fra et API for tiltakshistorikk, og produserer Kafka-hendelser for relevante tiltak.

## Datamodell
Tabell `arbeidssoekere`:

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

## Kafka
Applikasjonen konsumerer `paw.arbeidssokerperioder-v1` for å vedlikeholde listen av aktive arbeidssøkere (periode_avsluttet = null).
Når det kommer en avsluttet periode (periode_avsluttet != null) så slettes den tilhørende raden i `arbeidssoekere` tabellen.

## Skedulering
Det skal lages en enkel skeduleringsmekanisme som typisk kan kjøre med én times intervall. En arbeidssliste må persisteres og vedlikeholdes for å holde oversikt over hvilke arbeidssøkere som er prosessert. Listen må ha en status for hver arbeidssøker for å sikre at en arbeidssøker ikke prosesseres flere ganger. Ved feil kan man gjenoppta henting basert på arbeidslisten.

Skedulering kjører hvert 10 minutt og finner rader fra `arbeidssoekere` tabell med tidspunkt for `synk_sist_kjoert` eldre enn 2 timer.

### Oppgaver
* Finne en god lettvekts skeduleringsmekanisme for synkonieringsjobben.
* Utarbeide en løsning for en arbeidssliste som synkonieringsjobben kan bruke for å holde styr på fremdrift.
* Selv om skeduleringen kjører relativt ofte så må det være en mekansime som ser på tidsstempel for siste synkronisering per arbeidssøker for å se om den skal inkluderes i gjeldende kjøring. Skedulering kan kjøre hver 10 minutt mens synkronisering skal kun skje hver 2. time.
* Finne en løsning for race conditions når flere instanser jobber på samme arbeidssliste i paralell.

## Mulighetsrommet
Mulighetsrommet er en applikasjonsportfølje for oversikt over alle arbeidsmarkedstiltak Nav kan tilby.

Mulighetsrommet sin tiltakshistorikk-applikasjon tilbyr REST-API for å hente data basert på identitetsnummeret til arbeidssøkere.

* [Git Repo](https://github.com/navikt/mulighetsrommet)

### Mulighetsrommet Tiltakshistorikk
Applikasjon som tilbyr tiltakshistorikk via et REST-API.

OpenAPI-spec for REST-API finnes her: @app/tiltak-kafka-adapter/openapi/mulighetsrommet-tiltakshistorikk-api-spec.json

* [Git Repo](https://github.com/navikt/mulighetsrommet/tree/main/mulighetsrommet-tiltakshistorikk)
* [OpenAPI spec JSON](https://mulighetsrommet-tiltakshistorikk.intern.dev.nav.no/openapi.json)

### Oppgaver
* Opprette en domenemodell basert på OpenAPI-spec under en ny crate @domain/mulighetsrommet/ under stien @domain/mulighetsrommet/src/tiltakshistorikk.
* Opprette et nytt klient-bibliotek for henting fra API under en ny crate @lib/mulighetsrommet_client. Se eksempel på klient under @lib/paw_key_gen_client.
