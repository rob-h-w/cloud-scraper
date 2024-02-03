use strum_macros::Display;

#[derive(Debug, Display)]
pub(crate) enum PipelineError {
    Source(String),
    Sink(String),
}
