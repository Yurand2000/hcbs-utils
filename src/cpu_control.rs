use crate::prelude::*;
use crate::utils::*;

pub mod prelude {
    pub use super::intel::prelude as intel;
    pub use super::{
        hyperthreading_enabled,
        disable_hyperthreading,
        enable_hyperthreading,
        CpuFrequencyData,
        get_cpu_frequency,
        CpuFrequencyGovernorData,
        get_cpu_frequency_governor,
        set_cpu_frequency_governor,
        CpuIdleStates,
        get_cpu_idle_state,
        set_cpu_idle_state,
    };
}

pub mod intel;

const SMT_CONTROL_FILE: &'static str = "/sys/devices/system/cpu/smt/control";

pub fn hyperthreading_enabled() -> anyhow::Result<bool> {
    __read_file(SMT_CONTROL_FILE)
        .map(|str| str.trim() == "on")
}

pub fn disable_hyperthreading() -> anyhow::Result<()> {
    __write_file(SMT_CONTROL_FILE, "off")?;
    info!("Disabled hyperthreading");

    Ok(())
}

pub fn enable_hyperthreading() -> anyhow::Result<()> {
    __write_file(SMT_CONTROL_FILE, "on")?;

    info!("Enabled hyperthreading");
    Ok(())
}

pub struct CpuFrequencyData {
    pub min_frequency_mhz: u64,
    pub max_frequency_mhz: u64,
}

impl CpuFrequencyData {
    fn min_frequency_file(cpu: CpuID) -> String {
        format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/cpuinfo_min_freq")
    }

    fn max_frequency_file(cpu: CpuID) -> String {
        format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/cpuinfo_max_freq")
    }
}

pub fn get_cpu_frequency(cpu: CpuID) -> anyhow::Result<CpuFrequencyData> {
    let min_frequency_mhz = __read_file_parse(
        CpuFrequencyData::min_frequency_file(cpu), |freq| freq.trim().parse())?;

    let max_frequency_mhz = __read_file_parse(
        CpuFrequencyData::max_frequency_file(cpu), |freq| freq.trim().parse())?;

    Ok(CpuFrequencyData {
        min_frequency_mhz,
        max_frequency_mhz,
    })
}

pub struct CpuFrequencyGovernorData {
    pub governor: String,
    pub min_frequency_mhz: u64,
    pub max_frequency_mhz: u64,
}

impl CpuFrequencyGovernorData {
    pub fn fixed_frequency(frequency_mhz: u64) -> Self {
        Self {
            governor: format!("performance"),
            min_frequency_mhz: frequency_mhz,
            max_frequency_mhz: frequency_mhz,
        }
    }

    fn governor_file(cpu: CpuID) -> String {
        format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_governor")
    }

    fn min_frequency_file(cpu: CpuID) -> String {
        format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_min_freq")
    }

    fn max_frequency_file(cpu: CpuID) -> String {
        format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_max_freq")
    }
}

pub fn get_cpu_frequency_governor(cpu: CpuID) -> anyhow::Result<CpuFrequencyGovernorData> {
    let governor = __read_file(CpuFrequencyGovernorData::governor_file(cpu)).map(|gov| gov.trim().to_owned())?;

    let min_frequency_mhz = __read_file_parse(
        CpuFrequencyGovernorData::min_frequency_file(cpu), |freq| freq.trim().parse())?;

    let max_frequency_mhz = __read_file_parse(
        CpuFrequencyGovernorData::max_frequency_file(cpu), |freq| freq.trim().parse())?;

    Ok(CpuFrequencyGovernorData {
        governor,
        min_frequency_mhz,
        max_frequency_mhz,
    })
}

pub fn set_cpu_frequency_governor(cpu: CpuID, data: CpuFrequencyGovernorData)  -> anyhow::Result<()> {
    __write_file(
        CpuFrequencyGovernorData::governor_file(cpu),
        format!("{}", data.governor)
    )?;

    __write_file(
        CpuFrequencyGovernorData::min_frequency_file(cpu),
        format!("{}", data.min_frequency_mhz)
    )?;

    __write_file(
        CpuFrequencyGovernorData::max_frequency_file(cpu),
        format!("{}", data.max_frequency_mhz)
    )?;

    info!("Set Frequency Governor for cpu {cpu}: {} {} {}", data.governor, data.min_frequency_mhz, data.max_frequency_mhz);

    Ok(())
}

pub struct CpuIdleStates {
    states: Vec<bool>,
}

impl CpuIdleStates {
    pub fn disabled_for_cpu(cpu: CpuID) -> anyhow::Result<Self> {
        Ok(Self {
            states: vec![false; Self::get_cpu_num_states(cpu)?]
        })
    }

    fn get_cpu_num_states(cpu: CpuID) -> anyhow::Result<usize> {
        let dir = format!("/sys/devices/system/cpu/cpu{cpu}/cpuidle");
        let mut max_state = 0;
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?.file_name();
            let Some(name) = entry.to_str() else { anyhow::bail!("Error reading dir {dir} etries") };
            let Some(num) = name.strip_prefix("state") else { continue; };
            max_state = usize::max(max_state, num.parse::<usize>()?)
        }

        Ok(max_state + 1)
    }

    fn get_cpu_state_file(cpu: CpuID, state: usize) -> String {
        format!("/sys/devices/system/cpu/cpu{cpu}/cpuidle/state{state}/disable")
    }
}

pub fn get_cpu_idle_state(cpu: CpuID) -> anyhow::Result<CpuIdleStates> {
    Ok(CpuIdleStates {
        states: (0 .. CpuIdleStates::get_cpu_num_states(cpu)?)
            .map(|state| {
                __read_file_parse(
                    CpuIdleStates::get_cpu_state_file(cpu, state),
                    |disabled| {
                    let disabled = disabled.trim();
                    if disabled == "1" {
                        Ok(true)
                    } else if disabled == "0" {
                        Ok(false)
                    } else {
                        anyhow::bail!("unexpected value")
                    }
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?,
    })
}

pub fn set_cpu_idle_state(cpu: CpuID, data: CpuIdleStates) -> anyhow::Result<()> {
    let num_states = CpuIdleStates::get_cpu_num_states(cpu)?;

    if data.states.len() != num_states {
        anyhow::bail!("Incorrect number of CpuIdleStates for cpu {cpu}");
    }

    for (state, &disabled) in data.states.iter().enumerate() {
        __write_file(
            CpuIdleStates::get_cpu_state_file(cpu, state),
            if disabled { "1" } else { "0" },
        )?;
    }

    info!("Set cpu idle states for cpu {cpu}: {:?}", data.states);

    Ok(())
}