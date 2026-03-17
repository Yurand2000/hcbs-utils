use crate::prelude::*;
use crate::utils::*;

pub mod prelude {
    pub use super::{
        set_cgroup_period_us,
        set_cgroup_runtime_us,
        get_cgroup_period_us,
        get_cgroup_runtime_us,
    };
}

/// \[HCBS specific\] Set the cgroup server's period
pub fn set_cgroup_period_us(name: &str, period_us: u64) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);

    __write_file(
        format!("{path}/cpu.rt_period_us"),
        format!("{period_us}")
    )?;

    info!("Set period {period_us} us to {path}/cpu.rt_period_us");

    Ok(())
}

/// \[HCBS specific\] Set the cgroup server's runtime
pub fn set_cgroup_runtime_us(name: &str, runtime_us: u64) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);

    __write_file(
        format!("{path}/cpu.rt_runtime_us"),
        format!("{runtime_us}")
    )?;

    info!("Set runtime {runtime_us} us to {path}/cpu.rt_runtime_us");

    Ok(())
}

/// \[HCBS specific\] Get the cgroup server's period
pub fn get_cgroup_period_us(name: &str) -> anyhow::Result<u64> {
    __read_file_parse(
        format!("{}/cpu.rt_period_us", cgroup_abs_path(name)),
        |s| s.trim().parse::<u64>()
    )
}

/// \[HCBS specific\] Get the cgroup server's runtime
pub fn get_cgroup_runtime_us(name: &str) -> anyhow::Result<u64> {
    __read_file_parse(
        format!("{}/cpu.rt_runtime_us", cgroup_abs_path(name)),
        |s| s.trim().parse::<u64>()
    )
}