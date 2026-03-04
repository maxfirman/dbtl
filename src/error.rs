use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Usage(String),
    #[error("manifest.json not found at {path}")]
    MissingManifest { path: PathBuf },
    #[error("failed reading manifest at {path}: {source}")]
    ManifestRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed parsing manifest JSON at {path}: {source}")]
    ManifestParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("model '{model_name}' not found")]
    ModelNotFound { model_name: String },
    #[error("model name '{model_name}' is ambiguous. Candidates:\n{candidates}")]
    ModelAmbiguous {
        model_name: String,
        candidates: String,
    },
    #[error("self update failed: {0}")]
    SelfUpdate(String),
}

impl AppError {
    pub fn usage(msg: impl Into<String>) -> Self {
        Self::Usage(msg.into())
    }

    pub fn self_update(msg: impl Into<String>) -> Self {
        Self::SelfUpdate(msg.into())
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            _ => 1,
        }
    }
}
