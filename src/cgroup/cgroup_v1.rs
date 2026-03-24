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

    if let Err(err) = nix::mount::mount::<str, str, str, str>(
        None,
        CGROUP_ROOT,
        Some("cgroup"),
        nix::mount::MsFlags::empty(),
        None
    ) {
        error!("Error in mounting Cgroup FS: {err}");
        anyhow::bail!("Error in mounting Cgroup v1 FS: {err}");
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

    let cpu_dir = format!("{CGROUP_ROOT}/cpu");
    if let Err(err) =
        std::fs::create_dir(cpu_dir.as_str())
            .map_err(|err| -> anyhow::Error { err.into() })
            .and_then(|_| nix::mount::mount::<str, str, str, str>(
                None,
                cpu_dir.as_str(),
                Some("cgroup"),
                nix::mount::MsFlags::empty(),
                Some("cpu")
            ).map_err(|err| err.into()))
    {
        error!("Error in mounting Cgroup v1 CPU FS: {err}");
        anyhow::bail!("Error in mounting Cgroup v1 CPU FS: {err}");
    }

    info!("Mounted Cgroup v1 CPU FS");

    Ok(())
}

pub fn create_cgroup(name: &str) -> anyhow::Result<()> {
    super::__create_cgroup_common(name)?;
    info!("Created Cgroup {name}");

    Ok(())
}