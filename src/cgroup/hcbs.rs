use crate::prelude::*;
use crate::utils::*;

pub mod prelude {
    pub use super::{
        set_cgroup_period_us,
        set_cgroup_runtime_us,
        get_cgroup_period_us,
        get_cgroup_runtime_us,

        set_cgroup_period_us_multi,
        set_cgroup_runtime_us_multi,
        get_cgroup_period_us_multi,
        get_cgroup_runtime_us_multi,
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

// \[HCBS multicpu specific\] --------------------------------------------------
pub fn get_cgroup_runtime_us_multi(name: &str) -> anyhow::Result<Vec<(u64, Vec<CpuID>)>> {
    __read_file_parse(
        format!("{}/cpu.rt_runtime_us", cgroup_abs_path(name)),
        |str| parse_times(str).map(|times| pack_times_by_cpu_id(&times))
    )
}

pub fn get_cgroup_period_us_multi(name: &str) -> anyhow::Result<Vec<(u64, Vec<CpuID>)>> {
    __read_file_parse(
        format!("{}/cpu.rt_period_us", cgroup_abs_path(name)),
        |str| parse_times(str).map(|times| pack_times_by_cpu_id(&times))
    )
}

pub fn set_cgroup_runtime_us_multi(name: &str, runtimes_us: impl Iterator<Item = (u64, impl Iterator<Item = u32>)>) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);
    let data = packed_to_string(runtimes_us);

    __write_file(
        format!("{path}/cpu.rt_runtime_us"),
        &data
    )?;

    info!("Set runtimes {data} to {path}/cpu.rt_period_us");

    Ok(())
}

pub fn set_cgroup_period_us_multi(name: &str, periods_us: impl Iterator<Item = (u64, impl Iterator<Item = u32>)>) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);
    let data = packed_to_string(periods_us);

    __write_file(
        format!("{path}/cpu.rt_period_us"),
        &data
    )?;

    info!("Set periods {data} to {path}/cpu.rt_period_us");

    Ok(())
}

fn pack_times_by_cpu_id(times_us: &[u64]) -> Vec<(u64, Vec<CpuID>)> {
    use std::collections::HashMap;

    let mut times: Vec<(u64, CpuID)> =
        times_us.into_iter().enumerate()
            .map(|(cpu, time_us)| (*time_us, cpu as CpuID)).collect();
    times.sort_unstable_by_key(|(time_us, _)| *time_us);

    times.into_iter().fold(HashMap::<u64, Vec<CpuID>>::new(), |mut acc, (time_us, cpu)| {
        if time_us == 0 { return acc; }

        if let Some(cpus) = acc.get_mut(&time_us) {
            cpus.push(cpu);
        } else {
            acc.insert(time_us, vec![cpu]);
        }

        acc
    }).into_iter().collect()
}

fn packed_to_string(times_us: impl Iterator<Item = (u64, impl Iterator<Item = u32>)>) -> String {
    let mut times_string =
        times_us.map(|(time, cpus)| {
            let mut time_str = format!("{time} ");
            for cpu in cpus {
                time_str += &format!("{cpu},");
            }

            time_str.pop();
            time_str
        }).fold(String::new(), |acc, time_str| acc + &time_str + " ");

    times_string.pop();
    times_string
}

fn parse_times(times: String) -> anyhow::Result<Vec<u64>> {
    times.trim().split_ascii_whitespace()
        .map(|s| s.parse::<u64>().map_err(|err| err.into()))
        .collect()
}