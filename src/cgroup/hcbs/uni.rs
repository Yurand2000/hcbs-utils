use crate::prelude::*;
use crate::utils::*;

pub mod prelude {
    pub use either::Either;
    pub use super::{
        Max,
        get_cgroup_us,
        set_cgroup_us,
    };
}

#[derive(Clone, Copy)]
#[derive(Debug)]
pub struct Max;

impl std::fmt::Display for Max {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "max")
    }
}

/// \[HCBS specific\] Get the cgroup server's bandwidth
pub fn get_cgroup_us(name: &str) -> anyhow::Result<(Either<u64, Max>, u64)> {
    __read_file_parse(
        format!("{}/cpu.rt.max", cgroup_abs_path(name)),
        |s| {
            let mut iter = s.trim().split_ascii_whitespace();
            let Some((runtime, period)) =
                iter.next()
                    .and_then(|r| iter.next().map(|p| (r, p)))
                    .and_then(|v| if iter.next().is_none() { Some(v) } else { None } )
                else { anyhow::bail!("Error reading cpu.rt.max file") };

            let runtime =
                if runtime == "max" {
                    Either::Right(Max)
                } else {
                    Either::Left(runtime.parse::<u64>()?)
                };
            let period = period.parse::<u64>()?;

            Ok(( runtime, period ))
        }
    )
}

/// \[HCBS specific\] Set the cgroup server's bandwidth
pub fn set_cgroup_us(name: &str, runtime_us: Either<u64, Max>, period_us: u64) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);

    let runtime_us =
        match runtime_us {
            Either::Left(num) => &format!("{num}"),
            Either::Right(_) => "max",
        };

    __write_file(
        format!("{path}/cpu.rt.max"),
        format!("{runtime_us} {period_us}")
    )
        .map_err(|_| anyhow::format_err!("Error in writing {runtime_us}/{period_us} to {path}/cpu.rt.max"))?;

    info!("Set runtime/period {runtime_us}/{period_us} us to {path}/cpu.rt.max");

    Ok(())
}