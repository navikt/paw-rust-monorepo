use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Validering av felt '{field}' feilet")]
    FieldNotValid { instance: String, field: String },
}

impl ValidationError {
    pub fn field_not_valid(&self, instance: String, field: String) -> Self {
        Self::FieldNotValid { instance, field }
    }

    pub fn instance(&self) -> String {
        match self {
            Self::FieldNotValid { instance, .. } => instance.clone(),
        }
    }
}
