use crate::prelude::*;
use crate::utils::*;

pub mod prelude {
    pub use super::{
        set_cgroup_period_us_multi,
        set_cgroup_runtime_us_multi,
        get_cgroup_period_us_multi,
        get_cgroup_runtime_us_multi,
        set_cgroup_period_us_multi_str,
        set_cgroup_runtime_us_multi_str,
    };
}

// \[HCBS multicpu specific\] --------------------------------------------------
fn parse_times_on_get(times: String) -> anyhow::Result<Vec<(u64, Vec<CpuID>)>> {
    use std::collections::HashMap;

    let times: anyhow::Result<Vec<_>> =
        times.trim().split_ascii_whitespace()
        .map(|s| s.parse::<u64>().map_err(|err| err.into()))
        .collect();

    let mut times: Vec<(u64, CpuID)> =
        times?.into_iter().enumerate()
            .map(|(cpu, time_us)| (time_us, cpu as CpuID)).collect();
    times.sort_unstable_by_key(|(time_us, _)| *time_us);

    Ok(times.into_iter().fold(HashMap::<u64, Vec<CpuID>>::new(), |mut acc, (time_us, cpu)| {
        if time_us == 0 { return acc; }

        if let Some(cpus) = acc.get_mut(&time_us) {
            cpus.push(cpu);
        } else {
            acc.insert(time_us, vec![cpu]);
        }

        acc
    }).into_iter().collect())
}

/// \[HCBS MultiCPU Specific\] Get the cgroup server's runtime
pub fn get_cgroup_runtime_us_multi(name: &str) -> anyhow::Result<Vec<(u64, Vec<CpuID>)>> {
    __read_file_parse(
        format!("{}/cpu.rt_runtime_us", cgroup_abs_path(name)),
        |str| parse_times_on_get(str)
    )
}

/// \[HCBS MultiCPU Specific\] Get the cgroup server's period
pub fn get_cgroup_period_us_multi(name: &str) -> anyhow::Result<Vec<(u64, Vec<CpuID>)>> {
    __read_file_parse(
        format!("{}/cpu.rt_period_us", cgroup_abs_path(name)),
        |str| parse_times_on_get(str)
    )
}

/// \[HCBS MultiCPU Specific\] Set the cgroup server's runtime
pub fn set_cgroup_runtime_us_multi<I, J>(name: &str, runtimes_us: I) -> anyhow::Result<()>
    where I: IntoIterator<Item = (u64, J)>, J: IntoIterator<Item = CpuID>
{
    let path = cgroup_abs_path(name);
    let data = packed_to_string(runtimes_us.into_iter().map(|(time, cpus)| (time, cpus.into_iter())));

    __write_file(
        format!("{path}/cpu.rt_runtime_us"),
        &data
    )?;

    info!("Set runtimes {data} to {path}/cpu.rt_runtime_us");

    Ok(())
}

/// \[HCBS MultiCPU Specific\] Set the cgroup server's period
pub fn set_cgroup_period_us_multi<I, J>(name: &str, periods_us: I) -> anyhow::Result<()>
    where I: IntoIterator<Item = (u64, J)>, J: IntoIterator<Item = CpuID>
{
    let path = cgroup_abs_path(name);
    let data = packed_to_string(periods_us.into_iter().map(|(time, cpus)| (time, cpus.into_iter())));

    __write_file(
        format!("{path}/cpu.rt_period_us"),
        &data
    )?;

    info!("Set periods {data} to {path}/cpu.rt_period_us");

    Ok(())
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

// HCBS Specific setters from str ----------------------------------------------
fn parse_times_on_set(times_us: &str) -> anyhow::Result<Vec<(u64, CpuSetUnchecked)>> {
    use std::str::FromStr as _;
    use nom::Parser;
    use nom::multi::*;
    use nom::character::complete::*;
    use nom::combinator::*;

    let uint = || {
        digit1::<&str, ()>
            .map_res(|num: &str| num.parse::<u64>())
    };

    let cpu_set = || {
        recognize(many1(one_of("0123456789-,")))
            .map_res(|str: &str| CpuSetUnchecked::from_str(str))
    };

    let time_set = || {
        map((uint(), space1, cpu_set()), |(time, _, set)| (time, set))
    };

    separated_list1(space1, time_set()).parse(times_us)
        .map(|(_, data)| data)
        .map_err(|err| {
            log::error!("Parse error for time string: {}", times_us);
            err.into()
        })
}

/// \[HCBS MultiCPU Specific\] Set the cgroup server's runtime (from str)
pub fn set_cgroup_runtime_us_multi_str(name: &str, runtimes_us: &str) -> anyhow::Result<()> {
    set_cgroup_runtime_us_multi(name, parse_times_on_set(runtimes_us)?
        .into_iter().map(|(time, cpus)| (time, cpus.into_iter())) )
}

/// \[HCBS MultiCPU Specific\] Set the cgroup server's period (from str)
pub fn set_cgroup_period_us_multi_str(name: &str, periods_us: &str) -> anyhow::Result<()> {
    set_cgroup_period_us_multi(name, parse_times_on_set(periods_us)?
        .into_iter().map(|(time, cpus)| (time, cpus.into_iter())) )
}