use crate::utils::{
    __read_file_parse,
    __write_file,
};

pub mod prelude {
    pub use super::{
        PState,
        has_intel_pstate,
        get_pstate,
        set_pstate,
    };
}

pub struct PState {
    max_performance: u32,
    min_performance: u32,
    no_turbo: bool,
}

impl PState {
    pub fn fix_performance() -> Self {
        Self {
            min_performance: 100,
            max_performance: 100,
            no_turbo: true,
        }
    }
}

const MIN_PERF_FILE: &'static str = "/sys/devices/system/cpu/intel_pstate/min_perf_pct";
const MAX_PERF_FILE: &'static str = "/sys/devices/system/cpu/intel_pstate/max_perf_pct";
const NO_TURBO_FILE: &'static str = "/sys/devices/system/cpu/intel_pstate/no_turbo";
const STATUS_FILE: &'static str = "/sys/devices/system/cpu/intel_pstate/status";

pub fn has_intel_pstate() -> anyhow::Result<bool> {
    std::fs::exists(STATUS_FILE)
        .map_err(|err| anyhow::format_err!("Error in checking file {STATUS_FILE} existence: {err}"))
}

pub fn get_pstate() -> anyhow::Result<PState> {
    Ok(PState {
        max_performance: __read_file_parse(MAX_PERF_FILE, |data| data.trim().parse())?,
        min_performance: __read_file_parse(MIN_PERF_FILE, |data| data.trim().parse())?,
        no_turbo: __read_file_parse(NO_TURBO_FILE, |enabled| {
            let enabled = enabled.trim();
            if enabled == "1" {
                Ok(true)
            } else if enabled == "0" {
                Ok(false)
            } else {
                anyhow::bail!("unexpected value")
            }
        })?,
    })
}

pub fn set_pstate(pstate: PState) -> anyhow::Result<()> {
    __write_file(MAX_PERF_FILE, format!("{}", pstate.max_performance))?;
    __write_file(MIN_PERF_FILE, format!("{}", pstate.min_performance))?;
    __write_file(NO_TURBO_FILE, if pstate.no_turbo { "1" } else { "0" })?;

    info!("Set Intel PState: min {} max {} no_turbo {}", pstate.max_performance, pstate.min_performance, pstate.no_turbo);

    Ok(())
}