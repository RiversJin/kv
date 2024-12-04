use thiserror;

#[allow(dead_code)]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("syntax error")]
    Syntax,

    #[error("wrong number of argumentes of '{0}' command")]
    WrongArgNumber(String),

    #[error("WRONGTYPE Operation against a key holding the wrong kind of value")]
    WrongType,
    
    #[error("{0}")]
    Other(String),

    #[error("value is not an integer or out of range")]
    InvalidInteger(#[from] std::num::ParseIntError),
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;