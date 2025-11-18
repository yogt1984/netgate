use poem::http::StatusCode;
use poem::Error as PoemError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Unauthorized: missing or invalid tenant ID")]
    Unauthorized,
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<AppError> for PoemError {
    fn from(err: AppError) -> Self {
        PoemError::from_string(err.to_string(), err.status_code())
    }
}

