# Oversikt API
API som skal gi saksbehandlere oversikt over porteføljen for sitt kontor. Det er en aggregering av data på tvers av
Arbeidssøkerregisteret med fokus på arbeidssøkeres ledighet og andre viktige datapunkter. Skal kunne filtrere på
ledighet og egenvurdering nyere enn en gitt dato.

## Dataelementer
* Identitet - Vedlikehold ved hjelp av identitet-topic
* ~~ArbeidssøkerId?~~
* Navn
  * Hent fra PDL ved periode start
* Periode
* Kontortilhørlighet
  * Liste
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
* På-vegne-av -> Ny på-vegne-av-start så legges kilde til i listen (om den ikke finnes fra før). Tom liste = Arbeidssøkerregisteret har ansvaret selv.
  * Liste

## Spørre
* Ledighet nyere/eldre enn
* Egenvurdert nyere enn

## Sikkerhet
* Azure saksbehandler token med ident
* Tilgangsstyring
* Auditlogging? nei, trenger ikke auditlogge for listevisninger
