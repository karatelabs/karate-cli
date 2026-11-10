//! Error types and exit codes for the Karate CLI.

use thiserror::Error;

/// Exit codes as defined in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    /// Success
    Success = 0,
    /// General error
    GeneralError = 1,
    /// Configuration/setup error (not bootstrapped, invalid config)
    ConfigError = 2,
    /// Network error (download failed, manifest unreachable)
    NetworkError = 3,
    /// JRE error (missing, corrupt, launch failed)
    JreError = 4,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        code as i32
    }
}

impl ExitCode {
    /// Create an exit code from a JVM process exit code.
    /// JVM exit codes are passed through as 100 + code.
    #[allow(dead_code)]
    pub fn from_jvm(_code: i32) -> Self {
        // We represent JVM codes as GeneralError but the actual
        // process exit will use the raw code
        ExitCode::GeneralError
    }

    /// Get the raw exit code for JVM pass-through.
    pub fn jvm_passthrough(code: i32) -> i32 {
        if code == 0 {
            0
        } else {
            100 + code.abs().min(155) // Cap at 255
        }
    }
}

/// Karate CLI errors.
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum KarateError {
    #[error("Karate is not set up. Run 'karate setup --all' first.")]
    NotBootstrapped,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("JRE error: {0}")]
    Jre(String),

    #[error("Karate JAR not found at {0}")]
    JarNotFound(String),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("Unsupported platform: {os}-{arch}")]
    UnsupportedPlatform { os: String, arch: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl KarateError {
    /// Get the appropriate exit code for this error.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            KarateError::NotBootstrapped | KarateError::Config(_) => ExitCode::ConfigError,
            KarateError::Network(_)
            | KarateError::DownloadFailed(_)
            | KarateError::ChecksumMismatch { .. } => ExitCode::NetworkError,
            KarateError::Jre(_) => ExitCode::JreError,
            _ => ExitCode::GeneralError,
        }
    }
}

/// The exit code for an error: the first `KarateError` in its chain decides, and a failed
/// HTTP request is a network error.
pub fn exit_code_for(err: &anyhow::Error) -> ExitCode {
    for cause in err.chain() {
        if let Some(e) = cause.downcast_ref::<KarateError>() {
            return e.exit_code();
        }
        if cause.downcast_ref::<reqwest::Error>().is_some() {
            return ExitCode::NetworkError;
        }
    }
    ExitCode::GeneralError
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_bootstrapped_names_the_non_interactive_fix() {
        let msg = KarateError::NotBootstrapped.to_string();
        assert!(msg.contains("'karate setup --all'"), "{msg}");
    }

    #[test]
    fn exit_codes_follow_the_spec_through_context() {
        use anyhow::Context;
        let not_set_up: anyhow::Result<()> = Err(KarateError::NotBootstrapped.into());
        let not_set_up = not_set_up.context("run failed").unwrap_err();
        assert_eq!(exit_code_for(&not_set_up), ExitCode::ConfigError);

        let checksum = anyhow::Error::from(KarateError::ChecksumMismatch {
            file: "f".into(),
            expected: "a".into(),
            actual: "b".into(),
        });
        assert_eq!(exit_code_for(&checksum), ExitCode::NetworkError);
        assert_eq!(
            exit_code_for(&anyhow::anyhow!("anything else")),
            ExitCode::GeneralError
        );
    }

    #[tokio::test]
    async fn a_failed_request_is_a_network_error() {
        let err = reqwest::get("http://127.0.0.1:9/").await.unwrap_err();
        let err = anyhow::Error::from(err).context("Failed to start download");
        assert_eq!(exit_code_for(&err), ExitCode::NetworkError);
    }
}
