use crate::prelude::*;
use crate::utils::__shell;

pub mod prelude {
    pub use super::{
        mount_cgroup_fs,
        get_system_rt_period_us,
        get_system_rt_runtime_us,
        set_system_rt_period_us,
        set_system_rt_runtime_us,
        cgroup_abs_path,
        create_cgroup,
        delete_cgroup,
        cgroup_exists,
        cgroup_num_procs,
        cgroup_pids,
        get_cgroup_period_us,
        get_cgroup_runtime_us,
        set_cgroup_period_us,
        set_cgroup_runtime_us,
    };
}

const CGROUP_ROOT: &'static str = "/sys/fs/cgroup";

/// Get absolute path of a given cgroup
#[cfg(not(feature = "cgroup_v2"))]
pub fn cgroup_abs_path(name: &str) -> String {
    format!("{CGROUP_ROOT}/cpu/{name}")
}

/// Get absolute path of a given cgroup
#[cfg(feature = "cgroup_v2")]
pub fn cgroup_abs_path(name: &str) -> String {
    format!("{CGROUP_ROOT}/{name}")
}

/// Check whether the given cgroup exists
pub fn cgroup_exists(name: &str) -> bool {
    let path = cgroup_abs_path(name);
    let path = std::path::Path::new(&path);
    path.exists() && path.is_dir()
}

/// Get the number of processes assigned to the given cgroup
pub fn cgroup_num_procs(name: &str) -> anyhow::Result<usize> {
    let path = cgroup_abs_path(name);

    std::fs::read_to_string(format!("{path}/cgroup.procs"))
        .map(|procs| procs.lines().count())
        .map_err(|err| anyhow::format_err!("Error in reading {path}/cgroup.procs: {err}"))
}

/// Get the PIDs of the processes assigned to the given cgroup
pub fn cgroup_pids(name: &str) -> anyhow::Result<Vec<Pid>> {
    if !cgroup_exists(name) {
        return Err(anyhow::format_err!("Cgroup {name} does not exist"));
    }

    let path = cgroup_abs_path(name);
    std::fs::read_to_string(format!("{path}/cgroup.procs"))?.lines()
        .map(|line| line.parse::<u32>().map_err(|err| err.into()))
        .collect()
}

/// Mount cgroup filesystem and the cpu controller
pub fn mount_cgroup_fs() -> anyhow::Result<()> {
    __mount_cgroup_fs()?;
    __mount_cpu_fs()?;

    Ok(())
}

#[cfg(not(feature = "cgroup_v2"))]
pub fn __mount_cgroup_fs() -> anyhow::Result<()> {
    if __shell(&format!("mount | grep cgroup"))?.stdout.len() > 0 {
        debug!("Cgroup v1 FS already mounted");
        return Ok(());
    }

    if !__shell(&format!("mount -t tmpfs tmpfs {CGROUP_ROOT}"))?.status.success() {
        debug!("Error in mounting Cgroup FS");
        return Err(anyhow::format_err!("Error in mounting Cgroup v1 FS"));
    }

    debug!("Mounted Cgroup v1 FS");

    Ok(())
}

#[cfg(not(feature = "cgroup_v2"))]
pub fn __mount_cpu_fs() -> anyhow::Result<()> {
    let cpu_path = format!("{CGROUP_ROOT}/cpu");
    let cpu_path = std::path::Path::new(&cpu_path);
    if cpu_path.exists() && cpu_path.is_dir() {
        debug!("Cgroup CPU FS already mounted");
        return Ok(());
    }

    if !__shell(&format!("mkdir {CGROUP_ROOT}/cpu"))?.status.success() ||
        !__shell(&format!("mount -t cgroup -o cpu cpu-cgroup {CGROUP_ROOT}/cpu"))?.status.success()
    {
        debug!("Error in mounting Cgroup v1 CPU FS");
        return Err(anyhow::format_err!("Error in mounting Cgroup v1 CPU FS"));
    }

    debug!("Mounted Cgroup v1 CPU FS");

    Ok(())
}

#[cfg(feature = "cgroup_v2")]
pub fn __mount_cgroup_fs() -> anyhow::Result<()> {
    if __shell(&format!("mount | grep cgroup2"))?.stdout.len() > 0 {
        debug!("Cgroup v2 FS already mounted");
        return Ok(());
    }

    if !__shell(&format!("mount -t cgroup2 none {CGROUP_ROOT}"))?.status.success() {
        debug!("Error in mounting Cgroup v2 FS");
        return Err(anyhow::format_err!("Error in mounting Cgroup v2 FS"));
    }

    debug!("Mounted Cgroup v2 FS");

    Ok(())
}

#[cfg(feature = "cgroup_v2")]
pub fn __mount_cpu_fs() -> anyhow::Result<()> {
    __enable_cpu_contoller_v2(".")
}

#[cfg(feature = "cgroup_v2")]
pub fn __is_cpu_contoller_v2_enabled(name: &str) -> anyhow::Result<bool> {
    if !cgroup_exists(name) {
        return Err(anyhow::format_err!("Cgroup {name} does not exist"));
    }

    let controllers_path = format!("{CGROUP_ROOT}/{name}/cgroup.subtree_control");
    let controllers_path = std::path::Path::new(&controllers_path);
    if !controllers_path.exists() || !controllers_path.is_file() {
        return Err(anyhow::format_err!("Unexpected! Controllers file for cgroup {name} does not exist"));
    }

    Ok(
        std::fs::read_to_string(controllers_path)
        .map_err(|err| anyhow::format_err!("Error in reading controllers for cgroup {name}: {err}") )
        .contains("cpu")
    )
}

#[cfg(feature = "cgroup_v2")]
pub fn __enable_cpu_contoller_v2(name: &str) -> anyhow::Result<()> {
    if __is_cpu_contoller_v2_enabled(name)? { return Ok(()); }

    let controllers_path = format!("{CGROUP_ROOT}/{name}/cgroup.subtree_control");
    std::fs::write(controllers_path, "+cpu")
        .map_err(|err| anyhow::format_err!("Error in enabling CPU controller for cgroup {name}: {err}") )?;

    debug!("Enabled CPU controller for cgroup {name}");

    Ok(())
}

#[cfg(feature = "cgroup_v2")]
pub fn __enable_cpu_contoller_v2_recursive(name: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new(name);
    let ancestors: Vec<_> = path.ancestors()
        .filter(|ancestor| ancestor.as_os_str().is_empty())
        .collect();

    ancestors.into_iter().rev()
        .try_for_each(|ancestror| __enable_cpu_contoller_v2(ancestror.to_str().unwrap()))?;

    Ok(())
}

/// Read /proc/sys/kernel/sched_rt_period_us
pub fn get_system_rt_period_us() -> anyhow::Result<u64> {
    std::fs::read_to_string("/proc/sys/kernel/sched_rt_period_us")
        .map_err(|err| anyhow::format_err!("Error in reading from /proc/sys/kernel/sched_rt_period_us: {err}"))
    .and_then(|s| s.trim().parse::<u64>()
        .map_err(|err| anyhow::format_err!("Error in parsing /proc/sys/kernel/sched_rt_period_us: {err}")))
}

/// Read /proc/sys/kernel/sched_rt_runtime_us
pub fn get_system_rt_runtime_us() -> anyhow::Result<u64> {
    std::fs::read_to_string("/proc/sys/kernel/sched_rt_runtime_us")
        .map_err(|err| anyhow::format_err!("Error in reading from /proc/sys/kernel/sched_rt_runtime_us: {err}"))
    .and_then(|s| s.trim().parse::<u64>()
        .map_err(|err| anyhow::format_err!("Error in parsing /proc/sys/kernel/sched_rt_runtime_us: {err}")))
}

/// Write to /proc/sys/kernel/sched_rt_period_us
pub fn set_system_rt_period_us(period_us: u64) -> anyhow::Result<()> {
    std::fs::write("/proc/sys/kernel/sched_rt_period_us", format!("{period_us}"))
        .map_err(|err| anyhow::format_err!("Error in writing period {period_us} us to /proc/sys/kernel/sched_rt_runtime_us: {err}"))?;

    debug!("Set period {period_us} us to /proc/sys/kernel/sched_rt_runtime_us");

    Ok(())
}

/// Write to /proc/sys/kernel/sched_rt_runtime_us
pub fn set_system_rt_runtime_us(runtime_us: u64) -> anyhow::Result<()> {
    std::fs::write("/proc/sys/kernel/sched_rt_runtime_us", format!("{runtime_us}"))
        .map_err(|err| anyhow::format_err!("Error in writing runtime {runtime_us} us to /proc/sys/kernel/sched_rt_runtime_us: {err}"))?;

    debug!("Set runtime {runtime_us} us to /proc/sys/kernel/sched_rt_runtime_us");

    Ok(())
}

/// Create new cgroup
///
/// Notes:: creates all the cgroup hierarchy recursively if necessary
pub fn create_cgroup(name: &str) -> anyhow::Result<()> {
    mount_cgroup_fs()?;

    if name == "." { return Ok(()); }

    if cgroup_exists(name) {
        debug!("Cgroup {name} already exists");
        return Ok(());
    }

    let path = cgroup_abs_path(name);
    std::fs::create_dir_all(&path)
        .map_err(|err| anyhow::format_err!("Error in creating directory {path}: {err}"))?;

    #[cfg(feature = "cgroup_v2")]
    __enable_cpu_contoller_v2_recursive(name)?;

    debug!("Created Cgroup {name}");

    Ok(())
}

/// Delete cgroup
///
/// Tries to delete the given cgroup. A cgroup can be deleted only if
/// there are no active processes inside. The function tries to wait to try and
/// give the kernel time to perform necessary cleanups on exit of processes.
pub fn delete_cgroup(name: &str) -> anyhow::Result<()> {
    if name == "." { return Ok(()); }

    if !cgroup_exists(name) {
        debug!("Cgroup {name} does not already exist");
        return Ok(());
    }

    // Try to give the kernel some time to cleanup the system as this will
    // sometimes fail even if all the processes have been killed
    if cgroup_num_procs(name)? > 0 {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    if cgroup_num_procs(name)? > 0 {
        let procs = cgroup_pids(name)?;
        debug!("Cgroup {name} has active processes: {procs:?}");
        return Err(anyhow::format_err!("Cgroup {name} has active processes"));
    }

    let path = cgroup_abs_path(name);
    std::fs::remove_dir(&path)
        .map_err(|err| anyhow::format_err!("Error in destroying directory {path}: {err}"))?;

    debug!("Deleted Cgroup {name}");

    Ok(())
}

/// \[HCBS specific\] Set the cgroup server's period
pub fn set_cgroup_period_us(name: &str, period_us: u64) -> anyhow::Result<()> {
    use std::io::Write as _;

    let path = cgroup_abs_path(name);

    std::fs::OpenOptions::new().write(true)
        .open(format!("{path}/cpu.rt_period_us"))
        .map_err(|err| anyhow::format_err!("Error in opening file {path}/cpu.rt_period_us: {err}"))?
        .write_all(format!("{period_us}").as_bytes())
        .map_err(|err| anyhow::format_err!("Error in writing period {period_us} us to {path}/cpu.rt_period_us: {err}"))?;

    debug!("Set period {period_us} us to {path}/cpu.rt_period_us");

    Ok(())
}

/// \[HCBS specific\] Set the cgroup server's runtime
pub fn set_cgroup_runtime_us(name: &str, runtime_us: u64) -> anyhow::Result<()> {
    use std::io::Write as _;

    let path = cgroup_abs_path(name);

    std::fs::OpenOptions::new().write(true)
        .open(format!("{path}/cpu.rt_runtime_us"))
        .map_err(|err| anyhow::format_err!("Error in opening file {path}/cpu.rt_runtime_us: {err}"))?
        .write_all(format!("{runtime_us}").as_bytes())
        .map_err(|err| anyhow::format_err!("Error in writing runtime {runtime_us} us to {path}/cpu.rt_runtime_us: {err}"))?;

    debug!("Set runtime {runtime_us} us to {path}/cpu.rt_runtime_us");

    Ok(())
}

/// \[HCBS specific\] Get the cgroup server's period
pub fn get_cgroup_period_us(name: &str) -> anyhow::Result<u64> {
    let path = cgroup_abs_path(name);

    Ok(
        std::fs::read_to_string(format!("{path}/cpu.rt_period_us"))
            .map_err(|err| anyhow::format_err!("Error in reading from {path}/cpu.rt_period_us: {err}"))
        .and_then(|s| s.trim().parse::<u64>()
            .map_err(|err| anyhow::format_err!("Error in parsing {path}/cpu.rt_period_us: {err}")))?
    )
}

/// \[HCBS specific\] Get the cgroup server's runtime
pub fn get_cgroup_runtime_us(name: &str) -> anyhow::Result<u64> {
    let path = cgroup_abs_path(name);

    Ok(
        std::fs::read_to_string(format!("{path}/cpu.rt_runtime_us"))
            .map_err(|err| anyhow::format_err!("Error in reading from {path}/cpu.rt_runtime_us: {err}"))
        .and_then(|s| s.trim().parse::<u64>()
            .map_err(|err| anyhow::format_err!("Error in parsing {path}/cpu.rt_runtime_us: {err}")))?
    )
}