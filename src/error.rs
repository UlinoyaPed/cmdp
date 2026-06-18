use thiserror::Error;

#[derive(Debug, Error)]
pub enum CmdpError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("template error in command '{command_id}': {reason}")]
    Template { command_id: String, reason: String },
}
