use strum_macros::Display;

#[derive(Clone, Debug, Display)]
pub(crate) enum PipelineError {
    Source(String),
    Sink(String),
}
