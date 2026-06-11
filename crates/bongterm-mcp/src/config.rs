//! Manual MCP JSON config import + schema validation.
//! Committed config holds `${secret:NAME}` / `${env:NAME}` references only;
//! plaintext secret values are rejected.

use crate::McpServerConfig;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpConfigFile {
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("JSON parse error: {0}")]
    Parse(String),
    #[error("schema validation failed: {0}")]
    Schema(String),
    #[error(
        "forbidden install command for server '{server}': `npx -y` auto-install is not allowed"
    )]
    ForbiddenInstall { server: String },
    #[error("plaintext secret rejected for env var {var}: use ${{secret:NAME}} or ${{env:NAME}}")]
    PlaintextSecret { var: String },
}

fn looks_like_plaintext_secret(value: &str) -> bool {
    if value.starts_with("${secret:") || value.starts_with("${env:") {
        return false;
    }

    value
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .any(|seg| seg.len() >= 20 && seg.chars().any(|c| c.is_ascii_digit()))
}

impl McpConfigFile {
    pub fn import(json: &str) -> Result<Self, ConfigError> {
        let file: Self =
            serde_json::from_str(json).map_err(|e| ConfigError::Parse(e.to_string()))?;
        if file.servers.is_empty() {
            return Err(ConfigError::Schema("at least one server required".into()));
        }
        for server in &file.servers {
            if server.name.trim().is_empty() {
                return Err(ConfigError::Schema("server name must be non-empty".into()));
            }
            if server.argv.is_empty() || server.argv[0].trim().is_empty() {
                return Err(ConfigError::Schema(format!(
                    "server '{}' argv must be non-empty",
                    server.name
                )));
            }
            for window in server.argv.windows(2) {
                if window[0] == "npx" && window[1] == "-y" {
                    return Err(ConfigError::ForbiddenInstall {
                        server: server.name.clone(),
                    });
                }
            }
            for (var, value) in &server.env {
                if looks_like_plaintext_secret(value) {
                    return Err(ConfigError::PlaintextSecret { var: var.clone() });
                }
            }
        }
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_valid_config() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["node", "fs-server.js"], "env": [["ROOT", "${env:HOME}"]] }
        ]}"#;
        let file = McpConfigFile::import(json).expect("valid config must import");
        assert_eq!(file.servers.len(), 1);
        assert_eq!(file.servers[0].name, "fs");
    }

    #[test]
    fn rejects_empty_server_name() {
        let json = r#"{ "servers": [ { "name": "", "argv": ["node"], "env": [] } ]}"#;
        let err = McpConfigFile::import(json).unwrap_err();
        assert!(matches!(err, ConfigError::Schema(_)), "got {err:?}");
    }

    #[test]
    fn rejects_empty_argv() {
        let json = r#"{ "servers": [ { "name": "fs", "argv": [], "env": [] } ]}"#;
        let err = McpConfigFile::import(json).unwrap_err();
        assert!(matches!(err, ConfigError::Schema(_)), "got {err:?}");
    }

    #[test]
    fn rejects_plaintext_secret_in_env() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["node"], "env": [["TOKEN", "ghp_realLookingPlaintextValue1234567890"]] }
        ]}"#;
        let err = McpConfigFile::import(json).unwrap_err();
        assert!(
            matches!(err, ConfigError::PlaintextSecret { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn allows_secret_reference_in_env() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["node"], "env": [["TOKEN", "${secret:GITHUB_PAT}"]] }
        ]}"#;
        assert!(McpConfigFile::import(json).is_ok());
    }

    #[test]
    fn rejects_npx_dash_y_at_import() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["npx", "-y", "@scope/server"], "env": [] }
        ]}"#;
        let err = McpConfigFile::import(json).unwrap_err();
        assert!(
            matches!(err, ConfigError::ForbiddenInstall { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn allows_npx_without_dash_y_at_import() {
        let json = r#"{ "servers": [
            { "name": "fs", "argv": ["npx", "@scope/server"], "env": [] }
        ]}"#;
        assert!(McpConfigFile::import(json).is_ok());
    }
}
