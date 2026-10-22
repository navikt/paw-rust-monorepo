# Requests

## curl
```shell
ACCESS_TOKEN="";
REQUEST_URL=https://tilgangskontroll-v2-arbeidssoekerregisteret.intern.dev.nav.no/api/v1/tilgang/person;
REQUEST_BODY='{"tilgang":"LESE","navIdent":"Z994498","identitetsnummer":"17868199474"}';
curl -o /dev/null -s -w "DNS lookup: %{time_namelookup}s\nConnect: %{time_connect}s\nSSL handshake: %{time_appconnect}s\nStart transfer: %{time_starttransfer}s\nTotal time: %{time_total}s\n" -X POST -H "Authorization: Bearer ${ACCESS_TOKEN}" -H "Content-Type: application/json" -d $REQUEST_BODY $REQUEST_URL
```

```shell
ACCESS_TOKEN="";
REQUEST_URL=https://tilgangskontroll-v2-arbeidssoekerregisteret.intern.dev.nav.no/api/v1/tilgang/enhet;
REQUEST_BODY='{"navIdent":"Z994498","navEnhetId":"4154"}';
curl -o /dev/null -s -w "DNS lookup: %{time_namelookup}s\nConnect: %{time_connect}s\nSSL handshake: %{time_appconnect}s\nStart transfer: %{time_starttransfer}s\nTotal time: %{time_total}s\n" -X POST -H "Authorization: Bearer ${ACCESS_TOKEN}" -H "Content-Type: application/json" -d $REQUEST_BODY $REQUEST_URL
```