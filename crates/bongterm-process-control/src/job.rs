//! Windows JobObject-backed `ProcessGovernor`.
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::mem::size_of;
use std::sync::Mutex;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_JOB_MEMORY, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE, PROCESS_VM_READ,
};

use crate::{GovernorError, JobObjectCaps, ProcessGovernor, ProcessHandle, TerminationReason};

struct AttachedJob {
    job_raw: isize,
    caps: JobObjectCaps,
}

/// Production `ProcessGovernor` backed by one Windows `JobObject` per tracked process.
#[derive(Default)]
pub struct WindowsJobGovernor {
    attached: Mutex<HashMap<ProcessHandle, AttachedJob>>,
}

impl WindowsJobGovernor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            attached: Mutex::new(HashMap::new()),
        }
    }

    unsafe fn open_process(handle: ProcessHandle) -> Result<HANDLE, GovernorError> {
        unsafe {
            OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_SET_QUOTA | PROCESS_TERMINATE,
                false,
                handle.0,
            )
            .map_err(|e| GovernorError::JobObject(e.to_string()))
        }
    }

    fn build_limits(
        caps: JobObjectCaps,
    ) -> Result<JOBOBJECT_EXTENDED_LIMIT_INFORMATION, GovernorError> {
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        let mut flags = JOB_OBJECT_LIMIT(0);

        if caps.rss_bytes > 0 {
            info.JobMemoryLimit = usize::try_from(caps.rss_bytes)
                .map_err(|e| GovernorError::JobObject(e.to_string()))?;
            flags |= JOB_OBJECT_LIMIT_JOB_MEMORY;
        }
        if caps.child_proc_count > 0 {
            info.BasicLimitInformation.ActiveProcessLimit = caps.child_proc_count;
            flags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        }

        info.BasicLimitInformation.LimitFlags = flags;
        Ok(info)
    }

    fn set_limits(job: HANDLE, caps: JobObjectCaps) -> Result<(), GovernorError> {
        let mut info = Self::build_limits(caps)?;
        unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&raw mut info).cast(),
                u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                    .map_err(|e| GovernorError::JobObject(e.to_string()))?,
            )
            .map_err(|e| GovernorError::JobObject(e.to_string()))
        }
    }
}

impl Drop for WindowsJobGovernor {
    fn drop(&mut self) {
        if let Ok(mut attached) = self.attached.lock() {
            for (_, job) in attached.drain() {
                unsafe {
                    let _ = CloseHandle(HANDLE(job.job_raw as *mut _));
                }
            }
        }
    }
}

impl ProcessGovernor for WindowsJobGovernor {
    fn attach(&self, handle: ProcessHandle, caps: JobObjectCaps) -> Result<(), GovernorError> {
        let mut attached = self.attached.lock().unwrap();
        unsafe {
            let process = Self::open_process(handle)?;
            let job = CreateJobObjectW(None, None)
                .map_err(|e| GovernorError::JobObject(e.to_string()))?;
            let result = (|| {
                Self::set_limits(job, caps)?;
                AssignProcessToJobObject(job, process)
                    .map_err(|e| GovernorError::JobObject(e.to_string()))?;
                Ok::<(), GovernorError>(())
            })();
            let _ = CloseHandle(process);
            if let Err(err) = result {
                let _ = CloseHandle(job);
                return Err(err);
            }
            attached.insert(
                handle,
                AttachedJob {
                    job_raw: job.0 as isize,
                    caps,
                },
            );
            Ok(())
        }
    }

    fn update_caps(&self, handle: ProcessHandle, caps: JobObjectCaps) -> Result<(), GovernorError> {
        let mut attached = self.attached.lock().unwrap();
        let Some(job) = attached.get_mut(&handle) else {
            return Err(GovernorError::NotTracked(handle));
        };
        Self::set_limits(HANDLE(job.job_raw as *mut _), caps)?;
        job.caps = caps;
        Ok(())
    }

    fn sample_rss(&self, handle: ProcessHandle) -> Result<u64, GovernorError> {
        let attached = self.attached.lock().unwrap();
        if !attached.contains_key(&handle) {
            return Err(GovernorError::NotTracked(handle));
        }
        unsafe {
            let process = Self::open_process(handle)?;
            let mut counters = PROCESS_MEMORY_COUNTERS::default();
            let result = GetProcessMemoryInfo(
                process,
                &raw mut counters,
                u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>())
                    .map_err(|e| GovernorError::JobObject(e.to_string()))?,
            );
            let _ = CloseHandle(process);
            result.map_err(|e| GovernorError::JobObject(e.to_string()))?;
            Ok(counters.WorkingSetSize as u64)
        }
    }

    fn terminate(
        &self,
        handle: ProcessHandle,
        _reason: TerminationReason,
    ) -> Result<(), GovernorError> {
        let mut attached = self.attached.lock().unwrap();
        let Some(job) = attached.remove(&handle) else {
            return Err(GovernorError::NotTracked(handle));
        };
        unsafe {
            let handle = HANDLE(job.job_raw as *mut _);
            TerminateJobObject(handle, 1).map_err(|e| GovernorError::JobObject(e.to_string()))?;
            let _ = CloseHandle(handle);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WindowsJobGovernor;

    #[test]
    fn constructs_governor() {
        let _ = WindowsJobGovernor::new();
    }
}
