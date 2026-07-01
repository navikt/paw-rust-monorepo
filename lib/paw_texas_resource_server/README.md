# Auth-rammeverk for NAIS Texas Resource Server

## Konfig

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
