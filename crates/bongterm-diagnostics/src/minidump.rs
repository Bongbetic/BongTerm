//! Minidump writer port and mock implementation.

use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum MinidumpError {
    #[error("minidump write failed: {0}")]
    Write(String),
}

pub trait MinidumpWriter: Send + Sync {
    /// Write a minidump for `pid` into `dir`.
    ///
    /// # Errors
    ///
    /// Returns [`MinidumpError`] when the platform writer cannot create the dump.
    fn write_minidump(&self, pid: u32, dir: &Path) -> Result<PathBuf, MinidumpError>;
}

#[derive(Default)]
pub struct MockMinidumpWriter;

impl MinidumpWriter for MockMinidumpWriter {
    fn write_minidump(&self, pid: u32, dir: &Path) -> Result<PathBuf, MinidumpError> {
        Ok(dir.join(format!("bongterm-{pid}.dmp")))
    }
}

#[cfg(windows)]
pub struct WindowsMinidump;

#[cfg(windows)]
impl MinidumpWriter for WindowsMinidump {
    fn write_minidump(&self, pid: u32, dir: &Path) -> Result<PathBuf, MinidumpError> {
        std::fs::create_dir_all(dir).map_err(|e| MinidumpError::Write(e.to_string()))?;
        let path = dir.join(format!("bongterm-{pid}.dmp"));
        std::fs::write(&path, b"minidump placeholder")
            .map_err(|e| MinidumpError::Write(e.to_string()))?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_minidump_names_pid() {
        let writer = MockMinidumpWriter;
        let path = writer
            .write_minidump(1234, Path::new("crashes"))
            .expect("write mock");
        assert_eq!(path, PathBuf::from("crashes").join("bongterm-1234.dmp"));
    }
}
