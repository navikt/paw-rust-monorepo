# Plan for logikk og datamodell for oversikt over ledighet for arbeidssøkere

## Mål
Gi saksbehandlere oversikt over porteføljen for sitt kontor, med fokus på arbeidssøkeres ledighet og andre viktige
datapunkter. Skal kunne filtrere på kontor og ledighet nyere enn en gitt dato.

## Kildedata
* Arbeidssøkerperiode (periode) - Kafka-hendelser kommer på paw.arbeidssokerperioder-v1 topic. Én hendelse når en periode starter, og én når den avsluttes.
    * periode.id (periode_id) - Unik identifikator for perioden, genereres ved periode start.
    * periode.identitetsnummer - Identitet til arbeidssøker.
    * periode.startet.tidspunkt - Tidspunkt for når perioden ble startet.
    * periode.avsluttet.tidspunkt - Tidspunkt for når perioden ble avsluttet. periode.avsluttet er null ved periode startet hendelse.
* Bekreftelse (bekreftelse) - Kafka-hendelser kommer på paw.arbeidssoker-bekreftelse-v1 topic. Arbeidssøkere sender inn bekreftelse hver 14. dag på om de har jobbet eller ikke. Dette er kjent som en bekreftelseperiode.
    * bekreftelse.id (bekreftelse_id) - Unik identifikator for bekreftelsen, genereres ved innsendelse.
    * bekreftelse.periode_id - Identifikator for perioden bekreftelsen gjelder for.
    * bekreftelse.bekreftelsesloesning - Hvilken løsning som er brukt for å sende inn bekreftelsen (f.eks. "ARBEIDSSOEKERREGISTERET", "DAGPENGER", "FRISKMELDT_TIL_ARBEIDSFORMIDLING").
    * bekreftelse.svar.gjelder_fra - Tidspunkt for når bekreftelsen gjelder fra.
    * bekreftelse.svar.gjelder_til - Tidspunkt for når bekreftelsen gjelder til. Vil typisk være 14 dager etter gjelder fra tidspunkt.
    * bekreftelse.svar.har_jobbet_i_denne_perioden - Boolean som indikerer om arbeidssøker har jobbet i bekreftelseperioden.
* På-vegne-av (paa-vegne-av) - Kafka-hendelser kommer på paw.arbeidssoker-bekreftelse-paavegneav-v1 topic. Inneholder informasjon om hvem som har ansvar for å innhente bekreftelser fra spesifikke arbeidssøkere.
    * paa-vegne-av.periode_id - Identifikator for perioden på-vegne-av gjelder for.
    * paa-vegne-av.bekreftelsesloesning - Hvilken løsning som tar over ansvar for å hente inn bekreftelser for gitt periode.
* Kontortilhørlighet (kontortilhoerighet) - Kafka-hendelser kommer på dab.arbeidsoppfolgingskontortilordninger-v2 topic. Inneholder informasjon om hvilke kontor en person tilhører.
    * kontortilhoerighet.kontor_id - Unik identifikator for kontor.
    * kontortilhoerighet.kontor_navn - Navn på kontor.
    * kontortilhoerighet.kontor_type - Type kontor (f.eks. "ARBEIDSOPPFOLGING", "ARENA", "GEOGRAFISK_TILKNYTNING").
  * Identiteter (identiteter) - REST-kall til kafka-key-generator for å hente response med alle identiteter (gjeldende og historiske) for identitetsnummer.
      * response.arbeidssoeker_id - Unik identifikator for arbeidssøker.
      * response.identiteter - Identiteter for arbeidssøker. Gjeldende identitet har identitet.gjeldende==true. Andre identiteter er historiske og har identitet.gjeldende==false.
  * Persondata (persondata) - REST-kall til PDL for å hente response med persondata for identitetsnummer.
      * response.fornavn - Fornavn til arbeidssøker.
      * response.mellomnavn - Mellomnavn til arbeidssøker. Kan være null.
      * response.etternavn - Etternavn til arbeidssøker.

## Databasemodell
* Arbeidssoekere-tabell - Oversikt over arbeidssøkere, hvor det er én rad per arbeidssøker.
  * arbeidssoeker.id - Unik ID for rad.
  * arbeidssoeker.identitetsnummer - Hentes fra periode.identitetsnummer ved periode start.
  * arbeidssoeker.arbeidssoeker_id - Hentes via REST-kall til kafka-key-generator ved periode start.
  * arbeidssoeker.fornavn - Hentes via REST-kall til PDL ved periode start.
  * arbeidssoeker.mellomnavn - Hentes via REST-kall til PDL ved periode start. Kan være null.
  * arbeidssoeker.etternavn - Hentes via REST-kall til PDL ved periode start.
* Kartlegginger-tabell - Liste over kartlegginger per arbeidssøker. Hver kartlegging inneholder:
    * kartlegging.parent_id - Fremmednøkkel til arbeidssoeker.id.
    * kartlegging.periode_id - Hentes fra periode.id ved periode start.
    * kartlegging.arbeidssoeker_siden - Hentes fra periode.startet.tidspunkt ved periode start.
    * kartlegging.arbeidsledig_siden - Kalkulert tidspunkt for når arbeidssøker har vært ledig siden.
* Perioder-tabell - Liste over perioder per arbeidssøker. Hver periode inneholder:
      * periode.periode_id - Unik id for hver periode.
      * periode.startet_tidspunkt - Tidspunkt for når perioden startet.
      * periode.avsluttet_tidspunkt - Tidspunkt for når perioden ble avsluttet. Er null om perioden er aktiv.
* Bekreftelser-tabell - Liste over bekreftelser per arbeidssøker. Hver bekreftelse inneholder:
      * bekreftelse.bekreftelse_id - Unik id for hver bekreftelse.
      * bekreftelse.periode_id - Id til perioden bekreftelsen gjelder for.
      * bekreftelse.gjelder_fra_tidspunkt - Tidspunkt for når bekreftelsen gjelder fra.
      * bekreftelse.gjelder_til_tidspunkt - Tidspunkt for når bekreftelsen gjelder til.
      * bekreftelse.bekreftelsesløsning - Hvilken løsning som er brukt for å sende inn bekreftelsen.
      * bekreftelse.har_jobbet_i_denne_perioden - Boolean som indikerer om arbeidssøker har jobbet i bekreftelseperioden.
      * bekreftelse.vil_fortsette_som_arbeidssoeker - Boolean som indikerer om person vil fortsette som arbeidssøker.
* Kontortilhoerligheter-tabell - Liste over kontor arbeidssøker tilhører. Hver kontortilhørlighet inneholder:
    * kontortilhoerighet.kontor_id - Unik identifikator for kontor.
    * kontortilhoerighet.kontor_navn - Navn på kontor.
    * kontortilhoerighet.kontor_type - Type kontor (f.eks. "ARBEIDSOPPFOLGING", "ARENA", "GEOGRAFISK_TILKNYTNING").

## Fakta
* Tidspunktet i bekreftelse.svar.gjelder_fra for første bekreftelse er det samme som periode.startet.tidspunkt. Det er fundamentalt for logikken til bekreftelse.
* Når en arbeidssøker svarer bekreftelse.svar.har_jobbet_i_denne_perioden=false så betyr det av personen ikke har jobbet i tiden mellom bekreftelse.svar.gjelder_fra og bekreftelse.svar.gjelder_til, som ligger bakover i tid.

## Forretningslogikk
* Ved periode startet med periode.avsluttet==null
    * Lagre periode, insert eller update
    * Hent identiteter-response via REST-kall til kafka-key-generator
    * Om det ikke finnes noen eksisterende arbeidssoeker for identiteter-response.arbeidssoeker_id
        * Lag ny arbeidssoeker med data fra identiteter-response og PDL
        * Opprett ny kartlegging tilknyttet arbeidssoeker med kartlegging.periode_id=periode.periode_id og arbeidsledig_siden=null
    * Om det finnes en eksisterende arbeidssoeker for identiteter-response.arbeidssoeker_id
      * Om det ikke finnes noen eksisterende kartlegginger
          * Opprett ny kartlegging tilknyttet arbeidssoeker med kartlegging.periode_id=periode.periode_id og arbeidsledig_siden=null
      * Om det finnes eksisterende kartlegginger og forrige_kartlegging.arbeidsledig_siden==null eller ny_kartlegging.periode_startet_tidspunkt - forrige_kartlegging.periode_avsluttet_tidspunkt > "14 dager"
          * Opprett ny kartlegging tilknyttet arbeidssoeker med kartlegging.periode_id=periode.periode_id og arbeidsledig_siden=null
      * Om det finnes eksisterende kartlegginger og forrige_kartlegging.arbeidsledig_siden!=null og ny_kartlegging.periode_startet_tidspunkt - forrige_kartlegging.periode_avsluttet_tidspunkt < "14 dager"
          * Sett ny_kartlegging.arbeidsledig_siden=forrige_kartlegging.arbeidsledig_siden.
* Ved ny bekreftelse:
    * Lagre bekreftelse, insert eller update
    * Om det ikke finnes noen eksisterende kartlegginger for bekreftelse.periode_id og bekreftelse.svar.har_jobbet_i_denne_perioden=false
        * Opprett ny kartlegging tilknyttet arbeidssoeker med kartlegging.periode_id=bekreftelse.periode_id og arbeidsledig_siden=kartlegging.arbeidsledig_siden=bekreftelse.svar.gjelder_fra
    * Om det ikke finnes noen eksisterende kartlegginger for bekreftelse.periode_id og bekreftelse.svar.har_jobbet_i_denne_perioden=true
        * Opprett ny kartlegging tilknyttet arbeidssoeker med kartlegging.periode_id=bekreftelse.periode_id og arbeidsledig_siden=null
    * Om bekreftelse.svar.har_jobbet_i_denne_perioden=false og kartlegging.arbeidsledig_siden=null
        * Sett kartlegging.arbeidsledig_siden=bekreftelse.svar.gjelder_fra for kartlegging.periode_id=bekreftelse.periode_id
    * Om bekreftelse.svar.har_jobbet_i_denne_perioden=false og kartlegging.arbeidsledig_siden!=null
        * Ikke gjør noe
    * Om bekreftelse.svar.har_jobbet_i_denne_perioden=true
        * Sett kartlegging.arbeidsledig_siden=null for kartlegging.periode_id=bekreftelse.periode_id
