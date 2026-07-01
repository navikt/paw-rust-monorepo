# Auth-rammeverk for OAuth2 Resource Server

## Konfig

`auth_config.toml` for NAIS-miljø
```toml
[issuers.azure]
well_known_url = "${AZURE_APP_WELL_KNOWN_URL}"
client_id = "${AZURE_APP_CLIENT_ID}"

[issuers.tokenx]
well_known_url = "${TOKEN_X_WELL_KNOWN_URL}"
client_id = "${TOKEN_X_CLIENT_ID}"

[issuers.idporten]
well_known_url = "${IDPORTEN_WELL_KNOWN_URL}"
client_id = "${IDPORTEN_CLIENT_ID}"

[issuers.maskinporten]
well_known_url = "${MASKINPORTEN_WELL_KNOWN_URL}"
client_id = "${MASKINPORTEN_CLIENT_ID}"
```

`auth_config.toml` for lokal-miljø
```toml
[issuers.azure]
well_known_url = "http://localhost:8081/azure/.well-known/openid-configuration"
client_id = "paw-arbeidssoekerregisteret-api-kartlegging"

[issuers.tokenx]
well_known_url = "http://localhost:8081/tokenx/.well-known/openid-configuration"
client_id = "paw-arbeidssoekerregisteret-api-kartlegging"

[issuers.idporten]
well_known_url = "http://localhost:8081/idporten/.well-known/openid-configuration"
client_id = "paw-arbeidssoekerregisteret-api-kartlegging"

[issuers.maskinporten]
well_known_url = "http://localhost:8081/maskinporten/.well-known/openid-configuration"
client_id = "paw-arbeidssoekerregisteret-api-kartlegging"
```
