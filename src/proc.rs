use crate::prelude::*;

pub mod prelude {
    pub use super::{
        get_pid_cgroup,
        is_pid_in_cgroup,
        assign_pid_to_cgroup,
        kill_pid,
    };
}

/// Get the cgroup of the given PID
pub fn get_pid_cgroup(pid: Pid) -> anyhow::Result<String> {
    std::fs::read_to_string(format!("/proc/{}/cgroup", pid))
        .map_err(|err| err.into())
        .and_then(|str| {
            str.trim().strip_prefix("0::/")
                .ok_or(anyhow::format_err!("cgroup file should've started with 0::/"))
                .map(|str| {
                    if str.is_empty() {
                        ".".to_string()
                    } else {
                        str.to_string()
                    }
                })
        })
}

/// Check if the given PID is assigned to the given cgroup
pub fn is_pid_in_cgroup(name: &str, pid: Pid) -> anyhow::Result<bool> {
    if !cgroup_exists(name) {
        return Err(anyhow::format_err!("Cgroup {name} does not exist"));
    }

    let pid = format!("{pid}");
    let path = cgroup_abs_path(name);
    Ok(std::fs::read_to_string(format!("{path}/cgroup.procs"))?.lines()
        .find(|line| line == &pid).is_some())
}

/// Assign PID to the given cgroup
pub fn assign_pid_to_cgroup(name: &str, pid: Pid) -> anyhow::Result<()> {
    if !cgroup_exists(name) {
        return Err(anyhow::format_err!("Cgroup {name} does not exist"));
    }

    let path = cgroup_abs_path(name);
    std::fs::write(format!("{path}/cgroup.procs"), pid.to_string())
        .map_err(|err| anyhow::format_err!("Error in migrating task {pid} to cgroup {name}: {err}"))?;

    debug!("Migrated task {pid} to Cgroup {name}");

    Ok(())
}

/// Kill the given process
pub fn kill_pid(pid: Pid) {
    let system = sysinfo::System::new();
    let pid = sysinfo::Pid::from_u32(pid);

    system.process(pid).iter()
        .for_each(|proc| { proc.kill(); proc.wait(); });

    debug!("Killed pid {pid}");
}