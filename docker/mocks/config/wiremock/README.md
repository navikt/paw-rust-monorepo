# Wiremock

## Persontilgang

`POST /api/v1/tilgang/person` bruker separate mapping-filer med statiske responser.
Alle betingelsene i en mapping må stemme for at den skal gi tilgang.

| Fil | identitetsnummer | navIdent | tilgang | harTilgang |
| --- | --- | --- | --- | --- |
| `paw-tilgangskontroll-v2-person-skrive.json` | `01017012345` | `Z123456` | `LESE`, `SKRIVE` eller `LESE_SKRIVE` | `true` |
| `paw-tilgangskontroll-v2-person-lese.json` | `02017012345` | `Z123456` | `LESE` | `true` |
| `paw-tilgangskontroll-v2-person-default.json` | Alle andre kombinasjoner | | | `false` |

Request-feltet heter `tilgang`, ikke `type`. Manglende felt treffer default-mappingen.
Alle tre mappingene returnerer HTTP 200 med en boolsk `harTilgang`.

Legg til en ny mapping-fil for et nytt scenario. Bruk `matchesJsonPath` med
`equalTo` for hver feltverdi og `jsonBody` for responsen. Flere `bodyPatterns`
betyr AND. Et regex-uttrykk som `^(LESE|SKRIVE|LESE_SKRIVE)$` gir OR mellom
verdiene og matcher hele strengen.

Scenarioene har `priority: 1`, mens default-mappingen har `priority: 10`.
Lavest tall vinner, uavhengig av filnavn. Unngå overlappende scenarioer med samme
prioritet. Behold default-mappingen avgrenset til samme metode og endpoint.
Restart WireMock etter endringer i mapping-filene.

## hentIdenter
```json
{
  "variables": {
    "ident": "01017012345",
    "historisk": true
  },
  "query": "query($ident: ID!, $historisk: Boolean) {\n    hentIdenter(ident: $ident, historikk: $historisk) {\n        identer {\n            ident\n            gruppe\n            historisk\n        }\n    }\n}"
}
```
