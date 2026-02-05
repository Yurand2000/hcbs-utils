use crate::prelude::*;
use crate::utils::{
    __read_file,
    __read_file_parse,
};

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
    __read_file_parse(format!("/proc/{}/cgroup", pid),
        |str| {
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
        anyhow::bail!("Cgroup {name} does not exist");
    }

    let pid = format!("{pid}");
    let path = cgroup_abs_path(name);
    Ok(__read_file(format!("{path}/cgroup.procs"))?.lines()
        .find(|line| line == &pid).is_some())
}

/// Assign PID to the given cgroup
pub fn assign_pid_to_cgroup(name: &str, pid: Pid) -> anyhow::Result<()> {
    if !cgroup_exists(name) {
        error!("Cannot migrate {pid} to cgroup {name}: cgroup does not exist");
        anyhow::bail!("Cgroup {name} does not exist");
    }

    let path = cgroup_abs_path(name);
    std::fs::write(format!("{path}/cgroup.procs"), pid.to_string())
        .map_err(|err| anyhow::format_err!("Error in migrating task {pid} to cgroup {name}: {err}"))?;

    info!("Migrated task {pid} to cgroup {name}");

    Ok(())
}

/// Kill the given process
pub fn kill_pid(pid: Pid) -> anyhow::Result<()> {
    let pid = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), false);

    let Some(proc) = system.process(pid)
        else { warn!("Cannot kill PID {pid}: not found!"); return Ok(()); };

    let res = proc.kill_and_wait()
        .map_err(|err| anyhow::format_err!("Cannot kill PID {pid}: {err:?}"));

    info!("Killed PID {pid}");

    res?;

    Ok(())
}