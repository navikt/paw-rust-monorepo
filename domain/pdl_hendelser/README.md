# PDL Hendelser

## LEESAH
En hendelsestrøm av person-hendelser fra PDL.

Hendelsene er Avro-meldinger definert av skjemaene [her](https://github.com/navikt/pdl/tree/master/libs/contract-pdl-avro/src/main/avro/no/nav/person/pdl/leesah).
Disse må kopieres til `./avro/avdl/leesah` så kjøre `just leesah-avsc` for å generere avsc-filer til `./avro/avsc/leesah`.

## Diagnostikk ved Avro-feil

Deserialiseringsfeil viser hvilket steg som feilet: `create reader`, `decode`
eller `convert to Rust`. Ved dekodingsfeil viser meldingen også hvor mange bytes
reader hadde lest da feilen oppstod. Dette er ikke nødvendigvis byteposisjonen
der feilen startet.

Feilbeskrivelsene velges fra Avros strukturerte feilvarianter. De viser blant
annet komplette heltallstyper, forventet og faktisk Avro-type når disse er
tilgjengelige, og `std::io::ErrorKind` ved lesefeil. Eksempel:

```text
Kunne ikke deserialisere Avro-melding: decode (consumed 1 of 1 bytes): Cannot convert i64 to usize
```

Payload-verdier, feltstier og rå årsakskjeder tas ikke med. Fritekstfeil får
statiske beskrivelser, og ukartlagte feil vises som `Avro error (details omitted)`.
Dette gjelder også klassifiseringen av serialiserings- og skjemafeil.
Feilmeldingen alene fastslår ikke om årsaken er feil skjema, meldingsformat eller
Rust-type. Meldinger som feiler, håndteres som før.
