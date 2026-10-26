# Auth-rammeverk for OAuth2 Resource Server
OAuth2-basert autentisering. Kan enten benytte OAuth2 Resource Server med OIDC JWKS-cache, eller benytte
NAIS TEXAS Resource Server.

## OAuth2 Resource Server

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

## NAIS TEXAS Resource Server

`auth_config.toml` for NAIS-miljø
```toml
[texas]
introspection_endpoint = "${NAIS_TOKEN_INTROSPECTION_ENDPOINT}"

[[issuers]]
issuer = "${AZURE_OPENID_CONFIG_ISSUER}"
identity_provider = "entra_id"

[[issuers]]
issuer = "${TOKEN_X_ISSUER}"
identity_provider = "tokenx"

[[issuers]]
issuer = "${IDPORTEN_ISSUER}"
identity_provider = "idporten"

[[issuers]]
issuer = "${MASKINPORTEN_ISSUER}"
identity_provider = "maskinporten"
```

`auth_config.toml` for lokal-miljø
```toml
[texas]
introspection_endpoint = "http://localhost:8090/introspection"

[[issuers]]
issuer = "http://localhost:8081/azure"
identity_provider = "entra_id"

[[issuers]]
issuer = "http://localhost:8081/tokenx"
identity_provider = "tokenx"

[[issuers]]
issuer = "http://localhost:8081/idporten"
identity_provider = "idporten"

[[issuers]]
issuer = "http://localhost:8081/maskinporten"
identity_provider = "maskinporten"
```
