//! Conformance checks for minidump writers.

use std::path::Path;

use bongterm_diagnostics::minidump::MinidumpWriter;

/// # Panics
///
/// Panics when the writer does not return a `.dmp` path containing the target pid.
pub fn run_minidump_writer_conformance(writer: &impl MinidumpWriter) {
    let path = writer
        .write_minidump(99, Path::new("crashes"))
        .expect("writer should return a minidump path");
    assert!(path.to_string_lossy().contains("99"));
    assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("dmp"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bongterm_diagnostics::minidump::MockMinidumpWriter;

    #[test]
    fn mock_writer_satisfies_contract() {
        run_minidump_writer_conformance(&MockMinidumpWriter);
    }
}
