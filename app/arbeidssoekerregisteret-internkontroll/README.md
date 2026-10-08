> **KI-generert plan.** Dette er et arbeidsutkast basert på samtalen og en gjennomgang av kodebasen. Åpne spørsmål må avklares før de blir krav til løsningen.

# Internkontroll for arbeidssøkerregisteret

Arbeidssøkerregisteret er kilde for vedtak og beslutninger i Nav. Internkontroll skal oppdage integritetsavvik og gjøre det mulig å undersøke hva som skjedde i en meldingskjede. Kafka-topicene er kilden til sannhet for registeret.

Målet omfatter signaturvalidering, funn knyttet til traces og kontroll av forventede meldingsflyter. Opplysninger må kunne dokumenteres i klagesaker i minst ti år, trolig lenger. Nøyaktig oppbevaringstid og hva dokumentasjonsgrunnlaget skal inneholde, må avklares.

## Status i dag

Internkontroll abonnerer på åtte topics: perioder, opplysninger, profilering, på-vegne-av, bekreftelser, hendelseslogg, bekreftelseshendelseslogg og egenvurdering. Appen fletter meldinger fra topicene og bruker HWM til å følge framdriften per topic-partisjon. Meldingsprosessoren lagrer foreløpig ikke observasjoner eller integritetsfunn.

Modulen `paw_kafka::signing` kan validere en Kafka-melding uten å endre den. Den skiller mellom manglende signatur, ukjent nøkkel, ugyldig signatur og teknisk feil. Modulen har offentlige nøkler for historisk validering. En gyldig signatur viser ikke i seg selv at nøkkelen var tillatt på meldingens offset; den vurderingen ligger hos den som bruker modulen.

## Signeringsgrense

Signering ble innført etter at topicene allerede inneholdt usignerte meldinger. Internkontroll fastsetter derfor en egen grense for hver topic-partisjon.

Den første meldingen med minst én av headerne `x-paw-signature` eller `x-paw-signing-key-id` setter grenseoffseten. Det gjelder også hvis headeren er tom, den andre headeren mangler eller signaturen er ugyldig. Meldingen valideres og eventuelle feil registreres. Fra og med denne offseten er gyldig signatur påkrevd for alle meldinger på partisjonen.

Grensen lagres varig og flyttes ikke ved omstart eller replay. Meldinger før grensen kan være historisk usignerte uten at det er et signaturavvik. Meldinger etter grensen som mangler gyldig signatur, flagges.

Grensen, valideringsresultatet og HWM-oppdateringen skal inngå i samme Postgres-transaksjon. En feil ved lagring skal hindre at HWM flyttes forbi meldingen.

## Traces og funn

Alle meldinger forventes å ha trace-kontekst. Manglende eller ugyldig `traceparent` er et mistenkelig funn. Signaturfunn skal alltid kunne identifiseres med topic, partisjon og offset, også når trace mangler.

Internkontroll skal kunne flagge en trace når en melding i kjeden har ugyldig signatur. Trace-ID-en på en melding med ugyldig signatur er ikke verifisert. En kobling til trace basert på denne ID-en må derfor merkes som usikker, slik at en ugyldig melding ikke kan utgi seg for å tilhøre en annen trace uten at dette synes i funnet.

Hvordan funn knyttes til traces, og hvordan meldinger uten gyldig trace-kontekst følges opp, må beskrives nærmere før dette bygges.

## Kontekstuell validering

Vi har prosesser med kjent meldingsflyt: forventede meldingstyper, rekkefølge og forsinkelse mellom dem. En separat, asynkron jobb skal jevnlig vurdere åpne traces. Den skal kunne oppdage både at A har inntruffet uten forventet B innen en frist, og at B har inntruffet uten forventet A.

Vi starter med én konkret prosess. Regelen må angi hvilke meldinger som hører sammen, hva som avslutter en trace, hvilke forsinkelser som er tillatt, og når et mulig avvik blir et funn. Vurderingen må ta høyde for at meldinger fra ulike topics kan observeres i en annen rekkefølge enn den forventede prosessrekkefølgen.

## Lagring og dokumentasjon

Postgres er utgangspunktet for HWM, signeringsgrenser, åpne traces, funn og data som den asynkrone kontrollen må slå opp. Med flere titalls millioner meldinger i året og minst ti års dokumentasjonsbehov bør vi ikke uten videre lagre hele historikken med rå payload i aktive Postgres-tabeller.

Kompakte observasjoner kan støtte søk og kontroll, men de er ikke alene nok til å etterprøve hva en melding inneholdt i en klagesak. Vi må avklare hvor originalt meldingsinnhold bevares, hvordan det knyttes til observasjoner og funn, og hvordan det kan hentes fram etter at Kafka-retention er utløpt. Et mulig framtidig skille er aktive og søkbare data i Postgres, med lukkede traces og dokumentasjonsgrunnlag i GCP buckets.

Arkivering kan legges til senere, men lagringsmodellen må gjøre det mulig å finne igjen en melding via topic, partisjon og offset. Før data fjernes fra Postgres, må arkivkopien være skrevet og kontrollert. En bucket-skriving inngår ikke i HWM-transaksjonen; arkiveringsprosessen må derfor tåle krasj og gjentakelser.

For at signaturer skal kunne etterprøves over tid, må vi bevare det som trengs for å gjenskape de signerte bytesene, sammen med den relevante offentlige nøkkelen. Kravene til bevaring av payload, nøkkel, tidspunkt og headere må fastsettes før vi velger arkivformat. Oppbevaringstid, tilgang og gjenfinning må avklares ut fra dokumentasjonsbehovet.

## Kan internkontroll erstatte kafka-topic-backup?

Dette er et åpent spørsmål. Dagens `kafka-topic-backup` lagrer Kafka-payload, nøkkel, tidspunkt og headere i Postgres, men abonnerer på tre topics. Internkontroll leser åtte. Backupen gjør dessuten headere om til et JSON-objekt; det bevarer ikke nødvendigvis dupliserte headere eller de opprinnelige header-bytesene.

Før internkontroll eventuelt erstatter backupen, må vi avklare hvilke topics som inngår i dokumentasjonsgrunnlaget, hvilke meldingsdata som må bevares i opprinnelig form, og hvordan vi kontrollerer at arkivet er komplett og kan brukes i klagesaker. En erstatter må være verifisert mot disse kravene før backupen kan tas ut av drift.

## Foreslått rekkefølge

1. Lagre signeringsgrensen per topic-partisjon.
2. Valider hver melding mot grensen og lagre resultat og funn i HWM-transaksjonen. Test særlig meldingen som setter grensen, ufullstendige headere, omstart og replay.
3. Lagre trace-observasjoner og marker manglende eller ugyldig trace-kontekst.
4. Beskriv og prøv ut én regel for en kjent meldingsflyt. Kjør vurderingen asynkront over åpne traces.
5. Fastsett dokumentasjons- og oppbevaringskrav. Vurder deretter arkivering av lukkede traces og om internkontroll kan overta oppgaven til `kafka-topic-backup`.

## Grenser for integritetskontrollen

En gyldig signatur bekrefter de signerte delene av en observert melding mot en kjent offentlig nøkkel. Den beviser ikke alene at nøkkelen var tillatt på dette tidspunktet, at alle forventede meldinger ble produsert, eller at hele topic-historikken er bevart. Slike påstander krever egne regler og kontroller.

## Åpne spørsmål

- Hvilke av de åtte topicene må inngå i det varige dokumentasjonsgrunnlaget?
- Må dokumentasjonen bevare alle originale meldingsbytes og headere, eller finnes det andre godkjente kilder for deler av innholdet?
- Hvor lenge skal data bevares utover minst ti år, og hvordan skal historiske data kunne gjenfinnes i en klagesak?
- Hvordan avgjør vi om en signeringsnøkkel var tillatt på en gitt topic-partisjon og offset?
- Hvilken kjent meldingsflyt skal være den første vi kontrollerer?
- Hvilke krav må være oppfylt før internkontroll eventuelt kan erstatte `kafka-topic-backup`?
