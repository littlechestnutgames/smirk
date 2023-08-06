#[derive(Debug)]
pub enum CommandError {
    NoInput,
    ArgumentMismatch,
    Unknown,
    NoValidModeSpecified,
    InvalidTtlSpecified
}
