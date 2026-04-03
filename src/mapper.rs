use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct MapError(pub Box<dyn Error + Send + Sync>);

impl Display for MapError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for MapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

pub trait Mapper<I, O>: Send + Sync {
    fn map(&self, input: &I) -> Result<O, MapError>;
}

impl<F, I, O> Mapper<I, O> for F
where
    F: Fn(&I) -> Result<O, MapError> + Send + Sync,
{
    fn map(&self, input: &I) -> Result<O, MapError> {
        self(input)
    }
}
