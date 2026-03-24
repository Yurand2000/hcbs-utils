use crate::prelude::*;
use libc::{
    syscall,
    pid_t,
    sched_attr,
    SYS_sched_setattr,
    SYS_sched_getattr,
};

pub mod prelude {
    pub use super::{
        SchedPolicy,
        GetSchedPolicyError,
        SetSchedPolicyError,
        get_sched_policy,
        set_sched_policy,
    };
}

/// Scheduling Policy
#[derive(Debug)]
#[derive(Clone, Copy)]
#[derive(PartialEq, Eq)]
pub enum SchedPolicy {
    OTHER { nice: i32 },
    BATCH { nice: i32 },
    IDLE,
    FIFO(i32),
    RR(i32),
    DEADLINE {
        runtime_ms: u64,
        deadline_ms: u64,
        period_ms: u64,
    }
}

impl SchedPolicy {
    /// Builder for SCHED_OTHER at niceness zero
    pub fn other() -> Self { SchedPolicy::OTHER { nice: 0 }}

    pub fn is_other(&self) -> bool {
        match self {
            Self::OTHER { .. } => true,
            _ => false,
        }
    }

    pub fn is_fifo_rr(&self) -> bool {
        match self {
            Self::FIFO(_) => true,
            Self::RR(_) => true,
            _ => false,
        }
    }

    pub fn is_deadline(&self) -> bool {
        match self {
            Self::DEADLINE { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct GetSchedPolicyError {
    pid: Pid,
    error: SchedPolicyError,
}

#[derive(Debug)]
pub struct SetSchedPolicyError {
    pid: Pid,
    error: SchedPolicyError,
}

impl std::error::Error for GetSchedPolicyError {}

impl std::fmt::Display for GetSchedPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sched policy get error for PID {}: {}", self.pid, self.error)
    }
}

impl std::error::Error for SetSchedPolicyError {}

impl std::fmt::Display for SetSchedPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sched policy set error for PID {}: {}", self.pid, self.error)
    }
}

#[derive(Debug)]
pub enum SchedPolicyError {
    SyscallError(std::io::Error),
    UnknownPolicy(i32),
}

impl std::fmt::Display for SchedPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            SchedPolicyError::SyscallError(error) => write!(f, "Syscall Error: {error}"),
            SchedPolicyError::UnknownPolicy(policy)=> write!(f, "Unknown Policy: {policy}"),
        }
    }
}

impl TryFrom<sched_attr> for SchedPolicy {
    type Error = SchedPolicyError;

    fn try_from(value: sched_attr) -> Result<Self, Self::Error> {
        let res = match value.sched_policy as i32 {
            libc::SCHED_OTHER => SchedPolicy::OTHER { nice: value.sched_nice },
            libc::SCHED_BATCH => SchedPolicy::BATCH { nice: value.sched_nice },
            libc::SCHED_IDLE => SchedPolicy::IDLE,
            libc::SCHED_FIFO => SchedPolicy::FIFO( value.sched_priority as i32 ),
            libc::SCHED_RR => SchedPolicy::RR( value.sched_priority as i32 ),
            libc::SCHED_DEADLINE => SchedPolicy::DEADLINE {
                runtime_ms: value.sched_runtime,
                deadline_ms: value.sched_deadline,
                period_ms: value.sched_period,
            },
            val => { return Err(SchedPolicyError::UnknownPolicy(val)); }
        };

        Ok(res)
    }
}

impl Into<sched_attr> for SchedPolicy {
    fn into(self) -> sched_attr {
        let sched_policy = match self {
            SchedPolicy::OTHER { .. } => libc::SCHED_OTHER,
            SchedPolicy::BATCH { .. } => libc::SCHED_BATCH,
            SchedPolicy::IDLE => libc::SCHED_IDLE,
            SchedPolicy::FIFO(_) => libc::SCHED_FIFO,
            SchedPolicy::RR(_) => libc::SCHED_RR,
            SchedPolicy::DEADLINE { .. } => libc::SCHED_DEADLINE,
        } as u32;

        let sched_flags = match self {
            SchedPolicy::DEADLINE { .. } => libc::SCHED_FLAG_RESET_ON_FORK,
            _ => 0,
        } as u64;

        let sched_nice = match self {
            SchedPolicy::OTHER { nice } => nice,
            SchedPolicy::BATCH { nice } => nice,
            _ => 0,
        };

        let sched_priority = match self {
            SchedPolicy::FIFO(prio) => prio,
            SchedPolicy::RR(prio) => prio,
            _ => 0,
        } as u32;

        const MILLI_TO_NANO: u64 = 1000_000;
        let (sched_runtime, sched_deadline, sched_period) =
            match self {
                SchedPolicy::DEADLINE { runtime_ms, deadline_ms, period_ms }
                    => (
                        runtime_ms * MILLI_TO_NANO,
                        deadline_ms * MILLI_TO_NANO,
                        period_ms * MILLI_TO_NANO
                    ),
                _ => (0, 0, 0),
            };

        sched_attr {
            size: size_of::<sched_attr>() as u32,
            sched_policy,
            sched_flags,
            sched_nice,
            sched_priority,
            sched_runtime,
            sched_deadline,
            sched_period,
        }
    }
}

/// Get scheduling policy of the given PID
pub fn get_sched_policy(pid: Pid) -> Result<SchedPolicy, GetSchedPolicyError> {
    unsafe {
        let mut attr = sched_attr {
            size: 0,
            sched_policy: 0,
            sched_flags: 0,
            sched_nice: 0,
            sched_priority: 0,
            sched_runtime: 0,
            sched_deadline: 0,
            sched_period: 0,
        };

        let res =
            syscall(
                SYS_sched_getattr,
                pid                         as pid_t,
                &mut attr                   as *mut sched_attr,
                size_of::<sched_attr>()     as libc::c_uint,
                0                           as libc::c_uint,
            );

        if res != 0 {
            Err(GetSchedPolicyError { pid, error: SchedPolicyError::SyscallError(std::io::Error::last_os_error())})
        } else {
            attr.try_into()
                .map_err(|error| GetSchedPolicyError { pid, error })
        }
    }
}

/// Set the scheduling policy of the given PID
pub fn set_sched_policy(pid: Pid, policy: SchedPolicy) -> Result<(), SetSchedPolicyError> {
    let res;

    unsafe {
        let attr: sched_attr = policy.into();

        res =
            syscall(
                SYS_sched_setattr,
                pid                         as pid_t,
                &attr                       as *const sched_attr,
                0                           as libc::c_uint,
            );
    };

    if res != 0 {
        Err(SetSchedPolicyError { pid, error: SchedPolicyError::SyscallError(std::io::Error::last_os_error())})
    } else {
        info!("Set task {pid} sched policy to {policy:?}");
        Ok(())
    }
}