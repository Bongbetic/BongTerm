//! Runtime checks for forbidden implementation techniques.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForbiddenTechnique {
    DllInjection,
    HiddenConsoleScraping,
    UndocumentedSyscall,
    ProcessHollowing,
    KernelDriver,
    GlobalKeyboardHook,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessObservation {
    pub pid: u32,
    pub command_line: String,
}

pub trait ProcessTreeAuditor {
    fn scan(&self, observations: &[ProcessObservation]) -> Vec<ForbiddenTechnique>;
}

#[derive(Default)]
pub struct KeywordProcessTreeAuditor;

impl ProcessTreeAuditor for KeywordProcessTreeAuditor {
    fn scan(&self, observations: &[ProcessObservation]) -> Vec<ForbiddenTechnique> {
        let mut hits = Vec::new();
        for observation in observations {
            let cmd = observation.command_line.to_ascii_lowercase();
            if cmd.contains("createremotethread") || cmd.contains("writeprocessmemory") {
                hits.push(ForbiddenTechnique::DllInjection);
            }
            if cmd.contains("hidden-console-scrape") || cmd.contains("readconsoleoutput") {
                hits.push(ForbiddenTechnique::HiddenConsoleScraping);
            }
            if cmd.contains("ntdll!") || cmd.contains("ntcreate") {
                hits.push(ForbiddenTechnique::UndocumentedSyscall);
            }
            if cmd.contains("hollow") {
                hits.push(ForbiddenTechnique::ProcessHollowing);
            }
            if cmd.contains(".sys") || cmd.contains("kernel-driver") {
                hits.push(ForbiddenTechnique::KernelDriver);
            }
            if cmd.contains("setwindowshookex") || cmd.contains("keyboard hook") {
                hits.push(ForbiddenTechnique::GlobalKeyboardHook);
            }
        }
        hits.sort_by_key(|hit| *hit as u8);
        hits.dedup();
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auditor_flags_forbidden_process_tree_markers() {
        let auditor = KeywordProcessTreeAuditor;
        let hits = auditor.scan(&[
            ProcessObservation {
                pid: 1,
                command_line: "helper CreateRemoteThread WriteProcessMemory".to_string(),
            },
            ProcessObservation {
                pid: 2,
                command_line: "helper SetWindowsHookEx".to_string(),
            },
        ]);
        assert!(hits.contains(&ForbiddenTechnique::DllInjection));
        assert!(hits.contains(&ForbiddenTechnique::GlobalKeyboardHook));
    }

    #[test]
    fn normal_conpty_process_tree_has_no_hits() {
        let auditor = KeywordProcessTreeAuditor;
        let hits = auditor.scan(&[
            ProcessObservation {
                pid: 1,
                command_line: "bongterm.exe".to_string(),
            },
            ProcessObservation {
                pid: 2,
                command_line: "powershell.exe -NoProfile".to_string(),
            },
        ]);
        assert!(hits.is_empty());
    }
}
