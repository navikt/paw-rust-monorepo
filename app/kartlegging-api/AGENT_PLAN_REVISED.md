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
    * bekreftelse.svar.har_jobbet_i_denne_perioden - Boolean som indikerer om arbeidssøker har jobbet i forrige bekreftelseperiode.
* På-vegne-av (paa-vegne-av) - Kafka-hendelser kommer på paw.arbeidssoker-bekreftelse-paavegneav-v1 topic. Inneholder informasjon om hvem som har ansvar for å innhente bekreftelser fra spesifikke arbeidssøkere.
    * paa-vegne-av.periode_id - Identifikator for perioden på-vegne-av gjelder for.
    * paa-vegne-av.bekreftelsesloesning - Hvilken løsning som tar over ansvar for å hente inn bekreftelser for gitt periode.
* Kontortilhørlighet (kontortilhoerighet) - Kafka-hendelser kommer på dab.arbeidsoppfolgingskontortilordninger-v2 topic. Inneholder informasjon om hvilke kontor en person tilhører.
    * kontortilhoerighet.kontor_id - Unik identifikator for kontor.
    * kontortilhoerighet.kontor_navn - Navn på kontor.
    * kontortilhoerighet.kontor_type - Type kontor (f.eks. "ARBEIDSOPPFOLGING", "ARENA", "GEOGRAFISK_TILKNYTNING").

## Datamodell
Oversikt over arbeidssøkere, hvor det er én rad per arbeidssøker.
* arbeidssoeker.identitetsnummer - Hentes fra periode.identitetsnummer ved periode start.
* arbeidssoeker.arbeidssoeker_id - Hentes via REST-kall til kafka-key-generator ved periode start.
* arbeidssoeker.fornavn - Hentes via REST-kall til PDL ved periode start.
* arbeidssoeker.mellomnavn - Hentes via REST-kall til PDL ved periode start. Kan være null.
* arbeidssoeker.etternavn - Hentes via REST-kall til PDL ved periode start.
* Ledighetsperioder (ledighetsperiode) - Liste over ledighetsperioder per arbeidssøker. Hver ledighetsperiode inneholder:
    * ledighetsperiode.periode_id - Hentes fra periode.id ved periode start.
    * ledighetsperiode.periode_startet_tidspunkt - Hentes fra periode.startet.tidspunkt ved periode start.
    * ledighetsperiode.periode_avsluttet_tidspunkt - Hentes fra periode.avsluttet.tidspunkt ved periode avsluttet. Kan være null.
    * ledighetsperiode.bekreftelse_id - ID til siste bekreftelse for perioden. Kan være null om ingen bekreftelser er sendt inn.
    * ledighetsperiode.bekreftelsesløsning - Hvilken løsning som er brukt for å sende inn siste bekreftelse. Kan være null om ingen bekreftelser er sendt inn.
    * ledighetsperiode.bekreftelse_fra_tidspunkt - Hentes fra bekreftelse.svar.gjelder_fra for siste bekreftelse. Kan være null om ingen bekreftelser er sendt inn.
    * ledighetsperiode.har_jobbet_siste_periode - Hentes fra bekreftelse.svar.har_jobbet_i_denne_perioden for siste bekreftelse. Kan være null om ingen bekreftelser er sendt inn.
    * ledighetsperiode.ledig_fra_tidspunkt - Kalkulert tidspunkt for når arbeidssøker har vært ledig siden.
* Kontortilhørligheter (kontortilhoerighet) - Liste over kontor arbeidssøker tilhører. Hver kontortilhørlighet inneholder:
    * kontortilhoerighet.kontor_id
    * kontortilhoerighet.kontor_navn
    * kontortilhoerighet.kontor_type

## Fakta
* Tidspunktet i bekreftelse.svar.gjelder_fra for første bekreftelse er det samme som periode.startet.tidspunkt. Det er fundamentalt for logikken til bekreftelse.
* Når en arbeidssøker svarer bekreftelse.svar.har_jobbet_i_denne_perioden=false så betyr det av personen ikke har jobbet i tiden mellom bekreftelse.svar.gjelder_fra og bekreftelse.svar.gjelder_til, som ligger bakover i tid.

## Forretningslogikk
* Logikk for populering av ledighetsperiode.ledig_fra_tidspunkt:
    * Ved ny bekreftelse:
        * Om bekreftelse.svar.har_jobbet_i_denne_perioden=true, sett ledighetsperiode.ledig_fra_tidspunkt=null.
        * Om bekreftelse.svar.har_jobbet_i_denne_perioden=false:
            * Om dette er første bekreftelse på ny ledighetsperiode og forrige_ledighetsperiode.ledig_fra_tidspunkt!=null og ny_ledighetsperiode.periode_startet_tidspunkt - forrige_ledighetsperiode.periode_avsluttet_tidspunkt < "14 dager", sett ledighetsperiode.ledig_fra_tidspunkt=forrige_ledighetsperiode.ledig_fra_tidspunkt.
            * Ellers om ledighetsperiode.ledig_fra_tidspunkt=null, sett ledighetsperiode.ledig_fra_tidspunkt=bekreftelse.svar.gjelder_fra.
            * Ellers (ledighetsperiode.ledig_fra_tidspunkt!=null), ikke gjør noe.

## Endringslogg
* Revisjon 1: Presisert at bekreftelsesmodellen er bakoverrettet (gjelder_fra/gjelder_til ligger
  bakover i tid). Klargjort semantikken for har_jobbet_i_denne_perioden.
* Revisjon 2: Erstattet "første bekreftelse i perioden"-regel med tilstandsbasert logikk
  (ledig_fra=null → sett, ledig_fra!=null → beholdes). Presisert sammenligningsbase i
  kontinuitetsregelen til ny_periode.startet - forrige_periode.avsluttet.
* Revisjon 3: Slått sammen "ny bekreftelse"- og "ny periode"-blokkene til én prioritert
  IF-ELIF-kjede. Kontinuitetsregelen sjekkes nå eksplisitt først (mest spesifikk betingelse),
  deretter fallback til gjelder_fra, deretter ingen endring. Eliminerer regelkonflikt der begge
  blokker kunne treffe første bekreftelse på ny periode.
