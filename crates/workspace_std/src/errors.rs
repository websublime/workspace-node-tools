use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Unable to execute git process")]
    Execution,
    #[error("git failed with the following stdout: {stdout} stderr: {stderr}")]
    GitError { stdout: String, stderr: String },
}
