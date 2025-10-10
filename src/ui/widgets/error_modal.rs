use crate::errors::Error;
use std::fmt::{Display, Formatter};

#[derive(Default, Clone)]
pub struct ErrorModal {
    pub title: String,
    pub error_message: String,
    pub open: bool,
}

impl Display for ErrorModal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_message)
    }
}

impl<T> From<T> for ErrorModal
where
    T: AsRef<Error>,
{
    fn from(error_like: T) -> Self {
        let error_ref = error_like.as_ref();
        Self {
            title: error_ref.error_type(),
            error_message: error_ref.to_string(),
            open: true,
        }
    }
}
