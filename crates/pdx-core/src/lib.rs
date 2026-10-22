//! 純粋なモデル、検証、依存同一性、静的計画。I/Oを所有しない。
mod metadata;
mod model;
mod paths;
mod template;

pub use metadata::*;
pub use model::*;
pub use paths::*;
pub use template::*;

use serde::Serialize;

pub type Result<T> = std::result::Result<T, Diagnostic>;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub exit_code: u8,
    pub instance_path: Option<String>,
}

impl Diagnostic {
    pub fn new(code: &'static str, exit_code: u8, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            exit_code,
            instance_path: None,
        }
    }
    pub fn input(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, 2, message)
    }
    pub fn safety(message: impl Into<String>) -> Self {
        Self::new("unsafe_path", 4, message)
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Diagnostic {}
