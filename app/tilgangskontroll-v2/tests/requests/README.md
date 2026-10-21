# Requests

## curl
```shell
REQUEST_URL=https://kartlegging-arbeidssoekerregisteret.intern.dev.nav.no/api/v1/arbeidsledighet;
REQUEST_BODY='{"type":"TILKNYTTET_KONTOR","kontorId":"4154","paging":{"page":1,"pageSize":50,"sortOrder":"DESC"}}';
curl -X POST -H "Authorization: Bearer ${ACCESS_TOKEN}" -H "Content-Type: application/json" -d $REQUEST_BODY $REQUEST_URL
```

```shell
REQUEST_URL=https://kartlegging-arbeidssoekerregisteret.intern.dev.nav.no/api/v1/arbeidsledighet;
REQUEST_BODY='{"type":"TILKNYTTET_KONTOR","kontorId":"4154","paging":{"page":1,"pageSize":50,"sortOrder":"DESC"}}';
curl -o /dev/null -s -w "DNS lookup: %{time_namelookup}s\nConnect: %{time_connect}s\nSSL handshake: %{time_appconnect}s\nStart transfer: %{time_starttransfer}s\nTotal time: %{time_total}s\n" -X POST -H "Authorization: Bearer ${ACCESS_TOKEN}" -H "Content-Type: application/json" -d $REQUEST_BODY $REQUEST_URL
```