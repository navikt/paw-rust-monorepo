use serde_json::json;
use std::fmt;

pub(crate) struct AvroSchema {
    pub id: i32,
    pub version: i32,
    pub topic: &'static str,
    pub schema: &'static str,
}

impl AvroSchema {
    pub(crate) fn subject_path(&self) -> String {
        format!("/subjects/{}-value/versions/latest", self.topic)
    }

    pub(crate) fn schema_path(&self) -> String {
        format!("/schemas/ids/{}?deleted=true", self.id)
    }

    pub(crate) fn subject_response_body(&self) -> String {
        json!({
            "subject": format!("{}-value", self.topic),
            "version": self.version,
            "id": self.id,
            "schema": self.schema
        })
        .to_string()
    }

    pub(crate) fn schema_response_body(&self) -> String {
        json!({ "schema": self.schema }).to_string()
    }
}

impl fmt::Debug for AvroSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AvroSchema")
            .field("id", &self.id)
            .field("version", &self.version)
            .field("topic", &self.topic)
            .finish()
    }
}

pub(crate) fn avro_schemas() -> Vec<AvroSchema> {
    vec![
        AvroSchema {
            id: 1,
            version: 1,
            topic: PERIODE_TOPIC,
            schema: PERIODE_AVRO_SCHEMA,
        },
        AvroSchema {
            id: 2,
            version: 1,
            topic: OPPLYSNINGER_TOPIC,
            schema: OPPLYSNINGER_AVRO_SCHEMA,
        },
        AvroSchema {
            id: 3,
            version: 1,
            topic: PROFILERING_TOPIC,
            schema: PROFILERING_AVRO_SCHEMA,
        },
        AvroSchema {
            id: 4,
            version: 1,
            topic: EGENVURDERING_TOPIC,
            schema: EGENVURDERING_AVRO_SCHEMA,
        },
        AvroSchema {
            id: 5,
            version: 1,
            topic: BEKREFTELSE_TOPIC,
            schema: BEKREFTELSE_AVRO_SCHEMA,
        },
        AvroSchema {
            id: 6,
            version: 1,
            topic: BEKREFTELSE_PAAVEGNEAV_TOPIC,
            schema: BEKREFTELSE_PAAVEGNEAV_AVRO_SCHEMA,
        },
    ]
}

pub const PERIODE_TOPIC: &'static str = "paw.arbeidssokerperioder-v1";
pub const OPPLYSNINGER_TOPIC: &'static str = "paw.opplysninger-om-arbeidssoeker-v1";
pub const PROFILERING_TOPIC: &'static str = "paw.arbeidssoker-profilering-v1";
pub const EGENVURDERING_TOPIC: &'static str = "paw.arbeidssoeker-egenvurdering-v1";
pub const BEKREFTELSE_TOPIC: &'static str = "paw.arbeidssoker-bekreftelse-v1";
pub const BEKREFTELSE_PAAVEGNEAV_TOPIC: &'static str = "paw.arbeidssoker-bekreftelse-paavegneav-v1";

pub const PERIODE_AVRO_SCHEMA: &'static str = r#"
{
  "type": "record",
  "name": "Periode",
  "namespace": "no.nav.paw.arbeidssokerregisteret.api.v1",
  "fields": [
    {
      "name": "id",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "identitetsnummer",
      "type": {
        "type": "string",
        "avro.java.string": "String"
      }
    },
    {
      "name": "startet",
      "type": {
        "type": "record",
        "name": "Metadata",
        "fields": [
          {
            "name": "tidspunkt",
            "type": {
              "type": "long",
              "logicalType": "timestamp-millis"
            }
          },
          {
            "name": "utfoertAv",
            "type": {
              "type": "record",
              "name": "Bruker",
              "fields": [
                {
                  "name": "type",
                  "type": {
                    "type": "enum",
                    "name": "BrukerType",
                    "symbols": [
                      "UKJENT_VERDI",
                      "UDEFINERT",
                      "VEILEDER",
                      "SYSTEM",
                      "SLUTTBRUKER"
                    ],
                    "default": "UKJENT_VERDI"
                  }
                },
                {
                  "name": "id",
                  "type": {
                    "type": "string",
                    "avro.java.string": "String"
                  }
                },
                {
                  "name": "sikkerhetsnivaa",
                  "type": [
                    "null",
                    {
                      "type": "string",
                      "avro.java.string": "String"
                    }
                  ],
                  "default": null
                }
              ]
            }
          },
          {
            "name": "kilde",
            "type": {
              "type": "string",
              "avro.java.string": "String"
            }
          },
          {
            "name": "aarsak",
            "type": {
              "type": "string",
              "avro.java.string": "String"
            }
          },
          {
            "name": "tidspunktFraKilde",
            "type": [
              "null",
              {
                "type": "record",
                "name": "TidspunktFraKilde",
                "fields": [
                  {
                    "name": "tidspunkt",
                    "type": {
                      "type": "long",
                      "logicalType": "timestamp-millis"
                    }
                  },
                  {
                    "name": "avviksType",
                    "type": {
                      "type": "enum",
                      "name": "AvviksType",
                      "symbols": [
                        "UKJENT_VERDI",
                        "FORSINKELSE",
                        "RETTING",
                        "SLETTET",
                        "TIDSPUNKT_KORRIGERT"
                      ],
                      "default": "UKJENT_VERDI"
                    }
                  }
                ]
              }
            ],
            "default": null
          }
        ]
      }
    },
    {
      "name": "avsluttet",
      "type": [
        "null",
        "Metadata"
      ],
      "default": null
    }
  ]
}
"#;

pub const OPPLYSNINGER_AVRO_SCHEMA: &'static str = r#"
{
  "type": "record",
  "name": "OpplysningerOmArbeidssoeker",
  "namespace": "no.nav.paw.arbeidssokerregisteret.api.v4",
  "fields": [
    {
      "name": "id",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "periodeId",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "sendtInnAv",
      "type": {
        "type": "record",
        "name": "Metadata",
        "namespace": "no.nav.paw.arbeidssokerregisteret.api.v1",
        "fields": [
          {
            "name": "tidspunkt",
            "type": {
              "type": "long",
              "logicalType": "timestamp-millis"
            }
          },
          {
            "name": "utfoertAv",
            "type": {
              "type": "record",
              "name": "Bruker",
              "fields": [
                {
                  "name": "type",
                  "type": {
                    "type": "enum",
                    "name": "BrukerType",
                    "symbols": [
                      "UKJENT_VERDI",
                      "UDEFINERT",
                      "VEILEDER",
                      "SYSTEM",
                      "SLUTTBRUKER"
                    ],
                    "default": "UKJENT_VERDI"
                  }
                },
                {
                  "name": "id",
                  "type": {
                    "type": "string",
                    "avro.java.string": "String"
                  }
                },
                {
                  "name": "sikkerhetsnivaa",
                  "type": [
                    "null",
                    {
                      "type": "string",
                      "avro.java.string": "String"
                    }
                  ],
                  "default": null
                }
              ]
            }
          },
          {
            "name": "kilde",
            "type": {
              "type": "string",
              "avro.java.string": "String"
            }
          },
          {
            "name": "aarsak",
            "type": {
              "type": "string",
              "avro.java.string": "String"
            }
          },
          {
            "name": "tidspunktFraKilde",
            "type": [
              "null",
              {
                "type": "record",
                "name": "TidspunktFraKilde",
                "fields": [
                  {
                    "name": "tidspunkt",
                    "type": {
                      "type": "long",
                      "logicalType": "timestamp-millis"
                    }
                  },
                  {
                    "name": "avviksType",
                    "type": {
                      "type": "enum",
                      "name": "AvviksType",
                      "symbols": [
                        "UKJENT_VERDI",
                        "FORSINKELSE",
                        "RETTING",
                        "SLETTET",
                        "TIDSPUNKT_KORRIGERT"
                      ],
                      "default": "UKJENT_VERDI"
                    }
                  }
                ]
              }
            ],
            "default": null
          }
        ]
      }
    },
    {
      "name": "utdanning",
      "type": [
        "null",
        {
          "type": "record",
          "name": "Utdanning",
          "fields": [
            {
              "name": "nus",
              "type": {
                "type": "string",
                "avro.java.string": "String"
              }
            },
            {
              "name": "bestaatt",
              "type": [
                "null",
                {
                  "type": "enum",
                  "name": "JaNeiVetIkke",
                  "namespace": "no.nav.paw.arbeidssokerregisteret.api.v1",
                  "symbols": [
                    "JA",
                    "NEI",
                    "VET_IKKE"
                  ]
                }
              ],
              "default": null
            },
            {
              "name": "godkjent",
              "type": [
                "null",
                "no.nav.paw.arbeidssokerregisteret.api.v1.JaNeiVetIkke"
              ],
              "default": null
            }
          ]
        }
      ],
      "default": null
    },
    {
      "name": "helse",
      "type": [
        "null",
        {
          "type": "record",
          "name": "Helse",
          "namespace": "no.nav.paw.arbeidssokerregisteret.api.v1",
          "fields": [
            {
              "name": "helsetilstandHindrerArbeid",
              "type": "JaNeiVetIkke"
            }
          ]
        }
      ],
      "default": null
    },
    {
      "name": "jobbsituasjon",
      "type": {
        "type": "record",
        "name": "Jobbsituasjon",
        "namespace": "no.nav.paw.arbeidssokerregisteret.api.v1",
        "fields": [
          {
            "name": "beskrivelser",
            "type": {
              "type": "array",
              "items": {
                "type": "record",
                "name": "BeskrivelseMedDetaljer",
                "fields": [
                  {
                    "name": "beskrivelse",
                    "type": {
                      "type": "enum",
                      "name": "Beskrivelse",
                      "symbols": [
                        "UKJENT_VERDI",
                        "UDEFINERT",
                        "HAR_SAGT_OPP",
                        "HAR_BLITT_SAGT_OPP",
                        "ER_PERMITTERT",
                        "ALDRI_HATT_JOBB",
                        "IKKE_VAERT_I_JOBB_SISTE_2_AAR",
                        "AKKURAT_FULLFORT_UTDANNING",
                        "VIL_BYTTE_JOBB",
                        "USIKKER_JOBBSITUASJON",
                        "MIDLERTIDIG_JOBB",
                        "DELTIDSJOBB_VIL_MER",
                        "NY_JOBB",
                        "KONKURS",
                        "ANNET"
                      ],
                      "default": "UKJENT_VERDI"
                    }
                  },
                  {
                    "name": "detaljer",
                    "type": {
                      "type": "map",
                      "values": {
                        "type": "string",
                        "avro.java.string": "String"
                      },
                      "avro.java.string": "String"
                    }
                  }
                ]
              }
            }
          }
        ]
      }
    },
    {
      "name": "annet",
      "type": [
        "null",
        {
          "type": "record",
          "name": "Annet",
          "namespace": "no.nav.paw.arbeidssokerregisteret.api.v2",
          "fields": [
            {
              "name": "andreForholdHindrerArbeid",
              "type": [
                "null",
                "no.nav.paw.arbeidssokerregisteret.api.v1.JaNeiVetIkke"
              ],
              "default": null
            }
          ]
        }
      ],
      "default": null
    }
  ]
}
"#;

pub const PROFILERING_AVRO_SCHEMA: &'static str = r#"
{
  "type": "record",
  "name": "Profilering",
  "namespace": "no.nav.paw.arbeidssokerregisteret.api.v1",
  "fields": [
    {
      "name": "id",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "periodeId",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "opplysningerOmArbeidssokerId",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "sendtInnAv",
      "type": {
        "type": "record",
        "name": "Metadata",
        "fields": [
          {
            "name": "tidspunkt",
            "type": {
              "type": "long",
              "logicalType": "timestamp-millis"
            }
          },
          {
            "name": "utfoertAv",
            "type": {
              "type": "record",
              "name": "Bruker",
              "fields": [
                {
                  "name": "type",
                  "type": {
                    "type": "enum",
                    "name": "BrukerType",
                    "symbols": [
                      "UKJENT_VERDI",
                      "UDEFINERT",
                      "VEILEDER",
                      "SYSTEM",
                      "SLUTTBRUKER"
                    ],
                    "default": "UKJENT_VERDI"
                  }
                },
                {
                  "name": "id",
                  "type": {
                    "type": "string",
                    "avro.java.string": "String"
                  }
                },
                {
                  "name": "sikkerhetsnivaa",
                  "type": [
                    "null",
                    {
                      "type": "string",
                      "avro.java.string": "String"
                    }
                  ],
                  "default": null
                }
              ]
            }
          },
          {
            "name": "kilde",
            "type": {
              "type": "string",
              "avro.java.string": "String"
            }
          },
          {
            "name": "aarsak",
            "type": {
              "type": "string",
              "avro.java.string": "String"
            }
          },
          {
            "name": "tidspunktFraKilde",
            "type": [
              "null",
              {
                "type": "record",
                "name": "TidspunktFraKilde",
                "fields": [
                  {
                    "name": "tidspunkt",
                    "type": {
                      "type": "long",
                      "logicalType": "timestamp-millis"
                    }
                  },
                  {
                    "name": "avviksType",
                    "type": {
                      "type": "enum",
                      "name": "AvviksType",
                      "symbols": [
                        "UKJENT_VERDI",
                        "FORSINKELSE",
                        "RETTING",
                        "SLETTET",
                        "TIDSPUNKT_KORRIGERT"
                      ],
                      "default": "UKJENT_VERDI"
                    }
                  }
                ]
              }
            ],
            "default": null
          }
        ]
      }
    },
    {
      "name": "profilertTil",
      "type": {
        "type": "enum",
        "name": "ProfilertTil",
        "symbols": [
          "UKJENT_VERDI",
          "UDEFINERT",
          "ANTATT_GODE_MULIGHETER",
          "ANTATT_BEHOV_FOR_VEILEDNING",
          "OPPGITT_HINDRINGER"
        ],
        "default": "UKJENT_VERDI"
      }
    },
    {
      "name": "jobbetSammenhengendeSeksAvTolvSisteMnd",
      "type": "boolean"
    },
    {
      "name": "alder",
      "type": [
        "null",
        "int"
      ]
    }
  ]
}
"#;

pub const EGENVURDERING_AVRO_SCHEMA: &'static str = r#"
{
  "type": "record",
  "name": "Egenvurdering",
  "namespace": "no.nav.paw.arbeidssokerregisteret.api.v3",
  "fields": [
    {
      "name": "id",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "periodeId",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "profileringId",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "sendtInnAv",
      "type": {
        "type": "record",
        "name": "Metadata",
        "namespace": "no.nav.paw.arbeidssokerregisteret.api.v1",
        "fields": [
          {
            "name": "tidspunkt",
            "type": {
              "type": "long",
              "logicalType": "timestamp-millis"
            }
          },
          {
            "name": "utfoertAv",
            "type": {
              "type": "record",
              "name": "Bruker",
              "fields": [
                {
                  "name": "type",
                  "type": {
                    "type": "enum",
                    "name": "BrukerType",
                    "symbols": [
                      "UKJENT_VERDI",
                      "UDEFINERT",
                      "VEILEDER",
                      "SYSTEM",
                      "SLUTTBRUKER"
                    ],
                    "default": "UKJENT_VERDI"
                  }
                },
                {
                  "name": "id",
                  "type": {
                    "type": "string",
                    "avro.java.string": "String"
                  }
                },
                {
                  "name": "sikkerhetsnivaa",
                  "type": [
                    "null",
                    {
                      "type": "string",
                      "avro.java.string": "String"
                    }
                  ],
                  "default": null
                }
              ]
            }
          },
          {
            "name": "kilde",
            "type": {
              "type": "string",
              "avro.java.string": "String"
            }
          },
          {
            "name": "aarsak",
            "type": {
              "type": "string",
              "avro.java.string": "String"
            }
          },
          {
            "name": "tidspunktFraKilde",
            "type": [
              "null",
              {
                "type": "record",
                "name": "TidspunktFraKilde",
                "fields": [
                  {
                    "name": "tidspunkt",
                    "type": {
                      "type": "long",
                      "logicalType": "timestamp-millis"
                    }
                  },
                  {
                    "name": "avviksType",
                    "type": {
                      "type": "enum",
                      "name": "AvviksType",
                      "symbols": [
                        "UKJENT_VERDI",
                        "FORSINKELSE",
                        "RETTING",
                        "SLETTET",
                        "TIDSPUNKT_KORRIGERT"
                      ],
                      "default": "UKJENT_VERDI"
                    }
                  }
                ]
              }
            ],
            "default": null
          }
        ]
      }
    },
    {
      "name": "profilertTil",
      "type": {
        "type": "enum",
        "name": "ProfilertTil",
        "namespace": "no.nav.paw.arbeidssokerregisteret.api.v1",
        "symbols": [
          "UKJENT_VERDI",
          "UDEFINERT",
          "ANTATT_GODE_MULIGHETER",
          "ANTATT_BEHOV_FOR_VEILEDNING",
          "OPPGITT_HINDRINGER"
        ],
        "default": "UKJENT_VERDI"
      }
    },
    {
      "name": "egenvurdering",
      "type": "no.nav.paw.arbeidssokerregisteret.api.v1.ProfilertTil"
    }
  ]
}
"#;

pub const BEKREFTELSE_AVRO_SCHEMA: &'static str = r#"
{
  "type": "record",
  "name": "Bekreftelse",
  "namespace": "no.nav.paw.bekreftelse.melding.v1",
  "fields": [
    {
      "name": "periodeId",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "bekreftelsesloesning",
      "type": {
        "type": "enum",
        "name": "Bekreftelsesloesning",
        "namespace": "no.nav.paw.bekreftelse.melding.v1.vo",
        "symbols": [
          "UKJENT_VERDI",
          "ARBEIDSSOEKERREGISTERET",
          "DAGPENGER",
          "FRISKMELDT_TIL_ARBEIDSFORMIDLING"
        ],
        "default": "UKJENT_VERDI"
      }
    },
    {
      "name": "id",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "svar",
      "type": {
        "type": "record",
        "name": "Svar",
        "namespace": "no.nav.paw.bekreftelse.melding.v1.vo",
        "fields": [
          {
            "name": "sendtInnAv",
            "type": {
              "type": "record",
              "name": "Metadata",
              "fields": [
                {
                  "name": "tidspunkt",
                  "type": {
                    "type": "long",
                    "logicalType": "timestamp-millis"
                  }
                },
                {
                  "name": "utfoertAv",
                  "type": {
                    "type": "record",
                    "name": "Bruker",
                    "fields": [
                      {
                        "name": "type",
                        "type": {
                          "type": "enum",
                          "name": "BrukerType",
                          "symbols": [
                            "UKJENT_VERDI",
                            "UDEFINERT",
                            "VEILEDER",
                            "SYSTEM",
                            "SLUTTBRUKER"
                          ],
                          "default": "UKJENT_VERDI"
                        }
                      },
                      {
                        "name": "id",
                        "type": {
                          "type": "string",
                          "avro.java.string": "String"
                        }
                      },
                      {
                        "name": "sikkerhetsnivaa",
                        "type": [
                          "null",
                          {
                            "type": "string",
                            "avro.java.string": "String"
                          }
                        ],
                        "default": null
                      }
                    ]
                  }
                },
                {
                  "name": "kilde",
                  "type": {
                    "type": "string",
                    "avro.java.string": "String"
                  }
                },
                {
                  "name": "aarsak",
                  "type": {
                    "type": "string",
                    "avro.java.string": "String"
                  }
                }
              ]
            }
          },
          {
            "name": "gjelderFra",
            "type": {
              "type": "long",
              "logicalType": "timestamp-millis"
            }
          },
          {
            "name": "gjelderTil",
            "type": {
              "type": "long",
              "logicalType": "timestamp-millis"
            }
          },
          {
            "name": "harJobbetIDennePerioden",
            "type": "boolean"
          },
          {
            "name": "vilFortsetteSomArbeidssoeker",
            "type": "boolean"
          }
        ]
      }
    }
  ]
}
"#;

pub const BEKREFTELSE_PAAVEGNEAV_AVRO_SCHEMA: &'static str = r#"
{
  "type": "record",
  "name": "PaaVegneAv",
  "namespace": "no.nav.paw.bekreftelse.paavegneav.v1",
  "fields": [
    {
      "name": "periodeId",
      "type": {
        "type": "string",
        "logicalType": "uuid"
      }
    },
    {
      "name": "bekreftelsesloesning",
      "type": {
        "type": "enum",
        "name": "Bekreftelsesloesning",
        "namespace": "no.nav.paw.bekreftelse.paavegneav.v1.vo",
        "symbols": [
          "UKJENT_VERDI",
          "ARBEIDSSOEKERREGISTERET",
          "DAGPENGER",
          "FRISKMELDT_TIL_ARBEIDSFORMIDLING"
        ],
        "default": "UKJENT_VERDI"
      }
    },
    {
      "name": "handling",
      "type": [
        {
          "type": "record",
          "name": "Start",
          "namespace": "no.nav.paw.bekreftelse.paavegneav.v1.vo",
          "fields": [
            {
              "name": "intervalMS",
              "type": "long"
            },
            {
              "name": "graceMS",
              "type": "long"
            }
          ]
        },
        {
          "type": "record",
          "name": "Stopp",
          "namespace": "no.nav.paw.bekreftelse.paavegneav.v1.vo",
          "fields": [
            {
              "name": "fristBrutt",
              "type": "boolean",
              "default": false
            }
          ]
        }
      ]
    }
  ]
}
"#;
