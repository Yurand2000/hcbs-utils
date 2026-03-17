use crate::prelude::*;
use crate::utils::*;

/// Get absolute path of a given cgroup
pub fn cgroup_abs_path(name: &str) -> String {
    format!("{CGROUP_ROOT}/cpu/{name}")
}

pub fn __mount_cgroup_fs() -> anyhow::Result<()> {
    if __shell(&format!("mount | grep cgroup"))?.stdout.len() > 0 {
        info!("Cgroup v1 FS already mounted");
        return Ok(());
    }

    if !__shell(&format!("mount -t tmpfs tmpfs {CGROUP_ROOT}"))?.status.success() {
        error!("Error in mounting Cgroup FS");
        anyhow::bail!("Error in mounting Cgroup v1 FS");
    }

    info!("Mounted Cgroup v1 FS");

    Ok(())
}

pub fn __mount_cpu_fs() -> anyhow::Result<()> {
    let cpu_path = format!("{CGROUP_ROOT}/cpu");
    let cpu_path = std::path::Path::new(&cpu_path);
    if cpu_path.exists() && cpu_path.is_dir() {
        info!("Cgroup CPU FS already mounted");
        return Ok(());
    }

    if !__shell(&format!("mkdir {CGROUP_ROOT}/cpu"))?.status.success() ||
        !__shell(&format!("mount -t cgroup -o cpu cpu-cgroup {CGROUP_ROOT}/cpu"))?.status.success()
    {
        error!("Error in mounting Cgroup v1 CPU FS");
        anyhow::bail!("Error in mounting Cgroup v1 CPU FS");
    }

    info!("Mounted Cgroup v1 CPU FS");

    Ok(())
}

pub fn create_cgroup(name: &str) -> anyhow::Result<()> {
    super::__create_cgroup_common(name)?;
    info!("Created Cgroup {name}");

    Ok(())
}