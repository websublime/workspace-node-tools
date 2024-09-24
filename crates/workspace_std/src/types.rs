use crate::errors::GitError;

use std::result::Result;

pub type GitResult<T> = Result<T, GitError>;
