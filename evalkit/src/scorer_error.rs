use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

#[derive(Debug)]
#[non_exhaustive]
pub enum ScorerError {
    Timeout(Duration),
    InvalidInput(Box<dyn Error + Send + Sync>),
    ProviderError(Box<dyn Error + Send + Sync>),
    Internal(Box<dyn Error + Send + Sync>),
}

impl ScorerError {
    pub fn invalid_input<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::InvalidInput(Box::new(error))
    }

    pub fn provider<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::ProviderError(Box::new(error))
    }

    pub fn internal<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::Internal(Box::new(error))
    }
}

impl Display for ScorerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout(duration) => write!(f, "scorer timed out after {duration:?}"),
            Self::InvalidInput(error) | Self::ProviderError(error) | Self::Internal(error) => {
                Display::fmt(error, f)
            }
        }
    }
}

impl Error for ScorerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Timeout(_) => None,
            Self::InvalidInput(error) | Self::ProviderError(error) | Self::Internal(error) => {
                Some(error.as_ref())
            }
        }
    }
}
