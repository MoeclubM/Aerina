use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("{0}")]
    InvalidOperation(String),
    #[error("{0}")]
    NotFound(String),
}
