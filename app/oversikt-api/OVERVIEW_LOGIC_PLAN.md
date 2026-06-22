# Plan for logikk og datamodell for oversikt over ledighet for arbeidssøkere

## Mål
Gi saksbehandlere oversikt over porteføljen for sitt kontor, med fokus på arbeidssøkeres ledighet og andre viktige
datapunkter. Skal kunne filtrere på kontor og ledighet nyere enn en gitt dato.

## Kildedata
* Arbeidssøkerperiode - Hendelser kommer på paw.arbeidssokerperioder-v1 topic. Én hendelse når en periode starter, og én når den avsluttes.
    * Periode-ID - Unik identifikator for perioden, genereres ved periode start.
    * Identitet - Identitet til arbeidssøker.
    * Fra tidspunkt - Tidspunkt for når perioden starter.
    * Til tidspunkt - Tidspunkt for når perioden ble avsluttet. Er null ved periode startet hendelse.
* Bekreftelse - Hendelser kommer på paw.arbeidssoker-bekreftelse-v1 topic. Arbeidssøkere sender inn bekreftelse hver 14. dag på om de har jobbet eller ikke. Dette er kjent som en bekreftelse-periode.
    * Bekreftelse-ID - Unik identifikator for bekreftelsen, genereres ved innsendelse.
    * Periode-ID - Identifikator for perioden bekreftelsen gjelder for.
    * Bekreftelsesløsning - Hvilken løsning som er brukt for å sende inn bekreftelsen (f.eks. "ARBEIDSSOEKERREGISTERET", "DAGPENGER", "FRISKMELDT_TIL_ARBEIDSFORMIDLING").
    * Gjelder fra tidspunkt - Tidspunkt for når bekreftelsen gjelder fra.
    * Gjelder til tidspunkt - Tidspunkt for når bekreftelsen gjelder til. Vil typisk være 14 dager etter gjelder fra tidspunkt.
    * Har jobbet siste bekreftelse-periode? (bool)
* På-vegne-av - Hendelser kommer på paw.arbeidssoker-bekreftelse-paavegneav-v1 topic. Inneholder informasjon om hvem som har ansvar for å innhente bekreftelser fra spesifikke arbeidssøkere.
    * Periode-ID - Identifikator for perioden på-vegne-av gjelder for.
    * Bekreftelsesloesning - Hvilken løsning som tar over ansvar for å hente inn bekreftelser for gitt periode.
* Kontortilhørlighet - Hendelse kommer på dab.arbeidsoppfolgingskontortilordninger-v2 topic. Inneholder informasjon om hvilke kontor en person tilhører.
    * Kontor-ID - Unik identifikator for kontor.
    * Kontor-navn - Navn på kontor.
    * Kontor-type - Type kontor (f.eks. "ARBEIDSOPPFOLGING", "ARENA", "GEOGRAFISK_TILKNYTNING").

## Datamodell
Oversikt over arbeidssøkere, hvor det er én rad per arbeidssøker.
* Identitet - Mottas som del av periode start.
* ArbeidssøkerId - Hentes via REST-kall til kafka-key-generator ved periode start.
* Navn - Hentes via REST-kall til PDL ved periode start.
* Ledighet - Liste over ledighetsperioder per arbeidssøker. Hver ledighetsperiode inneholder:
    * Periode-ID
    * Bekreftelse-ID - ID til siste bekreftelse for perioden, null ved periode start.
    * Bekreftelsesløsning - Hvilken løsning som er brukt for å sende inn siste bekreftelse.
    * Bekreftet til tidspunkt - Samme som "gjelder fra tidspunkt" for siste bekreftelse.
    * Har jobbet siste bekreftelse-periode?
    * Ledig fra tidspunkt - Kalkulert tidspunkt for når arbeidssøker har vært ledig siden.
* Kontortilhørlighet - Liste over kontor arbeidssøker tilhører. Hver kontortilhørlighet inneholder:
    * Kontor-ID
    * Kontor-navn
    * Kontor-type

## Forretningslogikk
* Ledig fra tidspunkt
    * Settes av "gjelder fra tidspunkt" for første bekreftelse, om den inneholder "har ikke jobbet".
    * Nulles om en bekreftelse inneholder "har jobbet".
    * Nulles ved periode avsluttet.
    * Om svaret på første bekreftelse på ny periode er "har ikke jobbet" og det er mindre enn 14 dager siden "bekreftet til tidspunkt" så settes forrige "ledig fra tidspunkt".
