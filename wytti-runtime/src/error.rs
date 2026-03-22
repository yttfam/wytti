use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to load module: {0}")]
    Load(#[source] anyhow::Error),
    #[error("failed to instantiate module: {0}")]
    Instantiate(#[source] anyhow::Error),
    #[error("failed to execute: {0}")]
    Execute(#[source] anyhow::Error),
    #[error("no _start function found")]
    NoEntryPoint,
    #[error("execution timed out after {0:?}")]
    Timeout(Duration),
    #[error("memory limit exceeded")]
    MemoryLimitExceeded,
}
