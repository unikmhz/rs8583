use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum RS8583Error {
    #[error("ISO8583 parse error: {error}")]
    ParseError { error: String },
}

// TODO: FieldParseError with field refs
impl RS8583Error {
    pub fn parse_error<T: ToString>(error: T) -> Self {
        Self::ParseError {
            error: error.to_string(),
        }
    }
}
