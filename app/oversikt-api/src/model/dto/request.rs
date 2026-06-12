use crate::model::dto::kontor::KontorType;
use crate::model::parse::{enum_type_not_found, EnumTypeParseError};
use crate::model::sort::SortOrder;
use chrono::NaiveDate;
use errors::validation::ValidationError;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr, ToSchema,
)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueryType {
    Identitetsnummer,
    TilknyttetKontor,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum QueryRequest {
    #[serde(rename = "IDENTITETSNUMMER")]
    Identitetsnummer(IdentitetsnummerQueryRequest),
    #[serde(rename = "TILKNYTTET_KONTOR")]
    TilknyttetKontor(TilknyttetKontorQueryRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdentitetsnummerQueryRequest {
    pub identitetsnummer: String,
    pub paging: Option<PagingRequest>,
}

impl IdentitetsnummerQueryRequest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if let Some(paging) = &self.paging {
            if let Err(error) = paging.validate() {
                return Err(error);
            }
        }
        if self.identitetsnummer.len() < 11 {
            Err(ValidationError::StrengLengde(
                "identitetsnummer".to_string(),
                self.identitetsnummer.len() as i64,
            )
            .into())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TilknyttetKontorQueryRequest {
    pub kontor_id: String,
    pub kontor_type: Option<KontorType>,
    pub ledig_siden: Option<NaiveDate>,
    pub paging: Option<PagingRequest>,
}

impl TilknyttetKontorQueryRequest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if let Some(paging) = &self.paging {
            paging.validate()?
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagingRequest {
    pub page: i32,
    pub page_size: i32,
    pub sort_order: SortOrder,
}

impl PagingRequest {
    pub fn offset(&self) -> i32 {
        (self.page - 1) * self.page_size
    }

    pub fn limit(&self) -> i32 {
        self.page_size
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.page < 1 {
            Err(ValidationError::TallStoerelse("page".to_string(), self.page as i64).into())
        } else if self.page_size < 1 {
            Err(
                ValidationError::TallStoerelse("page_size".to_string(), self.page_size as i64)
                    .into(),
            )
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_identitetsnummer_query_request() {
        let query = IdentitetsnummerQueryRequest {
            identitetsnummer: "01017012345".to_string(),
            paging: Some(PagingRequest {
                page: 1,
                page_size: 10,
                sort_order: SortOrder::Descending,
            }),
        };

        let query_json: String =
            serde_json::to_string(&query).expect("Failed to deserialize request");
        print!("JSON: {}", query_json);

        let json = r#"
        {
            "type": "IDENTITETSNUMMER",
            "identitetsnummer": "01017012345",
            "paging": {
                "page": 1,
                "pageSize": 10,
                "sortOrder": "DESC"
            }
        }
        "#;

        let request: QueryRequest =
            serde_json::from_str(json).expect("Failed to deserialize request");

        match request {
            QueryRequest::Identitetsnummer(query) => {
                assert_eq!(query.identitetsnummer, "01017012345");
                assert!(query.paging.is_some());
                let paging = query.paging.unwrap();
                assert_eq!(paging.page, 1);
                assert_eq!(paging.page_size, 10);
                assert_eq!(paging.sort_order, SortOrder::Descending);
            }
            _ => panic!("Wrong query"),
        }
    }

    #[test]
    fn test_deserialize_identitetsnummer_query_tilknyttet_kontor() {
        let json = r#"
        {
            "type": "TILKNYTTET_KONTOR",
            "kontorId": "12345",
            "ledigSiden": "2026-01-01",
            "paging": {
                "page": 3,
                "pageSize": 77,
                "sortOrder": "ASC"
            }
        }
        "#;

        let request: QueryRequest =
            serde_json::from_str(json).expect("Failed to deserialize request");

        match request {
            QueryRequest::TilknyttetKontor(query) => {
                assert_eq!(query.kontor_id, "12345");
                assert!(query.ledig_siden.is_some());
                let ledig_siden = query.ledig_siden.unwrap();
                assert_eq!(ledig_siden.to_string(), "2026-01-01");
                assert!(query.paging.is_some());
                let paging = query.paging.unwrap();
                assert_eq!(paging.page, 3);
                assert_eq!(paging.page_size, 77);
                assert_eq!(paging.sort_order, SortOrder::Ascending);
            }
            _ => panic!("Wrong query"),
        }
    }
}
