use std::error::Error;

use strum_macros::Display;

#[derive(Debug, Display)]
pub(crate) enum PipelineError {
    Create,
}

impl From<Box<dyn Error>> for PipelineError {
    fn from(_: Box<dyn Error>) -> Self {
        PipelineError::Create
    }
}
