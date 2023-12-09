use crate::arguments::ArgumentError;

pub struct FrameworkError;

impl From<ArgumentError> for FrameworkError {
    fn from(_: ArgumentError) -> Self {
        Self {}
    }
}