use crate::model::dto::response::SortOrder;
use crate::model::error::validation_error::ValidationError;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct QueryRequest {
    pub identitetsnummer: String,
    pub paging: Option<PagingRequest>,
}

impl QueryRequest {
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

#[derive(Clone, Debug, Deserialize, ToSchema)]
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
