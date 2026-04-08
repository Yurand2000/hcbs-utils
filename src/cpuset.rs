use crate::prelude::*;

pub mod prelude {
    pub use super::{
        CpuSet,
        CpuSetUnchecked,
        CpuSetBuildError,
        get_cpuset_to_pid,
        set_cpuset_to_pid,
    };
}

/// Set of valid CPUs on the current machine
#[derive(Debug)]
#[derive(Clone)]
#[derive(PartialEq)]
pub struct CpuSet {
    cpus: Vec<CpuID>,
}

#[derive(Debug)]
pub enum CpuSetBuildError {
    IO(std::io::Error),
    ParseError(String),
    UnavailableCPU(CpuID),
    UnavailableCPUs,
}

impl std::fmt::Display for CpuSetBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CpuSet creation error: ")?;

        match self {
            CpuSetBuildError::IO(error) => write!(f, "IO error: {error}"),
            CpuSetBuildError::ParseError(error) => write!(f, "Parse error: {error}"),
            CpuSetBuildError::UnavailableCPU(cpu) => write!(f, "Requesting unavailable cpu {cpu}"),
            CpuSetBuildError::UnavailableCPUs => write!(f, "Requesting more CPUs than available ones"),
        }
    }
}

impl std::error::Error for CpuSetBuildError {}

impl CpuSet {
    /// Create a CpuSet with only the given CPU, if available
    pub fn single(cpu: CpuID) -> Result<CpuSet, CpuSetBuildError>  {
        let all = CpuSet::all()?;

        if all.cpus.contains(&cpu) {
            Ok(CpuSet { cpus: vec![cpu] })
        } else {
            Err(CpuSetBuildError::UnavailableCPU(cpu))
        }
    }

    /// Create an empty CpuSet
    pub fn empty() -> CpuSet {
        CpuSet { cpus: Vec::with_capacity(0) }
    }

    /// Create a CpuSet containing all cpus
    pub fn all() -> Result<CpuSet, CpuSetBuildError> {
        use std::str::FromStr as _;

        let online_cpus = std::fs::read_to_string("/sys/devices/system/cpu/online")
            .map_err(|err| CpuSetBuildError::IO(err))?;
        let cpuset = CpuSetUnchecked::from_str(&online_cpus)
            .map_err(|err| CpuSetBuildError::ParseError(err))?;

        Ok(CpuSet { cpus: cpuset.cpus })
    }

    /// Create a CpuSet containing any given number of CPUs
    pub fn any_subset(num_cpus: u64) -> Result<CpuSet, CpuSetBuildError> {
        let all = CpuSet::all()?;

        if num_cpus as usize > all.cpus.len() {
            return Err(CpuSetBuildError::UnavailableCPUs);
        }

        Ok(CpuSet {
            cpus: all.cpus.into_iter().take(num_cpus as usize).collect()
        })
    }

    /// Get if a CPU is in the set
    pub fn has_cpu(&self, cpu: CpuID) -> bool {
        self.cpus.iter().any(|&my_cpu| my_cpu == cpu)
    }

    /// Return the number of CPUs in the set
    pub fn num_cpus(&self) -> usize {
        self.cpus.len()
    }

    /// Returns an iterator over the CPU set
    pub fn iter(&self) -> impl Iterator<Item = &CpuID> {
        self.cpus.iter()
    }

    /// Returns a mutable iterator over the CPU set
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut CpuID> {
        self.cpus.iter_mut()
    }

    /// Creates a consuming iterator over the CPU set
    pub fn into_iter(self) -> impl Iterator<Item = CpuID> {
        self.cpus.into_iter()
    }

    /// Return the number of configured CPUs in the system
    pub fn system_cpus() -> usize {
        unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) as usize }
    }

    /// Return the number of online CPUs in the system
    pub fn online_cpus() -> usize {
        unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize }
    }
}

/// Set of CPUs
///
/// Differs from [CpuSet] as it is not yet known if the machine has such CPUs
/// available.
#[derive(Debug)]
#[derive(Clone)]
#[derive(PartialEq)]
pub struct CpuSetUnchecked {
    cpus: Vec<CpuID>,
}

impl CpuSetUnchecked {
    /// Create an empty CpuSet
    pub fn empty() -> Self {
        Self { cpus: Vec::with_capacity(0) }
    }

    /// Add a CPU to the set
    pub fn add_cpu(mut self, cpu: CpuID) -> Self {
        if !self.cpus.contains(&cpu) {
            self.cpus.push(cpu);
        }

        self
    }

    /// Remove a CPU from the set
    pub fn remove_cpu(mut self, cpu: CpuID) -> Self {
        match self.cpus.iter().position(|elem| elem == &cpu) {
            Some(i) => { self.cpus.swap_remove(i); },
            None => (),
        };

        self
    }

    /// Get if a CPU is in the set
    pub fn has_cpu(&self, cpu: CpuID) -> bool {
        self.cpus.iter().any(|&my_cpu| my_cpu == cpu)
    }

    /// Get the number of CPUs in the set
    pub fn num_cpus(&self) -> usize {
        self.cpus.len()
    }

    /// Returns an iterator over the CPU set
    pub fn iter(&self) -> impl Iterator<Item = &CpuID> {
        self.cpus.iter()
    }

    /// Returns a mutable iterator over the CPU set
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut CpuID> {
        self.cpus.iter_mut()
    }

    /// Creates a consuming iterator over the CPU set
    pub fn into_iter(self) -> impl Iterator<Item = CpuID> {
        self.cpus.into_iter()
    }
}

impl std::str::FromStr for CpuSet {
    type Err = CpuSetBuildError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match CpuSetUnchecked::from_str(s) {
            Ok(cpus) => cpus.try_into(),
            Err(err) => Err(CpuSetBuildError::ParseError(err)),
        }
    }
}

impl std::str::FromStr for CpuSetUnchecked {
    type Err = String;

    fn from_str<'a>(s: &'a str) -> Result<Self, Self::Err> {
        use nom::Parser;
        use nom::bytes::complete::*;
        use nom::branch::*;
        use nom::multi::*;
        use nom::character::complete::*;
        use nom::combinator::*;

        let single_parser = || map_res(digit1::<&str, ()>, |s: &str| s.parse::<CpuID>());
        let single_parser_pair = map(single_parser(), |cpu| (cpu, cpu) );
        let range_parser = map_res(
            (
                single_parser(),
                tag("-"),
                single_parser()
            ),
            |(min, _, max)| {
                if min > max {
                    Err(format!("Range error"))
                } else {
                    Ok((min, max))
                }
            }
        );

        let separator_parser = map((tag(","), multispace0), |_| ());
        let mut parser = map(
            separated_list1(
                separator_parser,
                alt((range_parser, single_parser_pair))
            ),
            |pairs: Vec<(CpuID, CpuID)>| {
                let mut out: Vec<CpuID> = Vec::new();
                for pair in pairs.into_iter() {
                    for cpu in pair.0 ..= pair.1 {
                        out.push(cpu);
                    }
                }

                out
            }
        );

        Ok(CpuSetUnchecked {
            cpus: parser.parse(s).map_err(|err| format!("{err}"))?.1
        })
    }
}

impl TryInto<CpuSet> for CpuSetUnchecked {
    type Error = CpuSetBuildError;

    fn try_into(self) -> Result<CpuSet, Self::Error> {
        let all = CpuSet::all()?;

        for cpu in &self.cpus {
            if !all.cpus.contains(&cpu) {
                return Err(CpuSetBuildError::UnavailableCPU(*cpu));
            }
        }

        Ok(CpuSet { cpus: self.cpus })
    }
}

fn display_cpus(cpus: &[CpuID], f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[")?;

    let mut iter = cpus.iter().peekable();
    if iter.peek().is_some() {
        let cpu = iter.next().unwrap();
        write!(f, "{cpu}")?;

        for cpu in iter {
            write!(f, ", {cpu}")?;
        }
    }

    write!(f, "]")?;
    Ok(())
}

impl std::fmt::Display for CpuSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_cpus(&self.cpus, f)
    }
}

impl std::fmt::Display for CpuSetUnchecked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_cpus(&self.cpus, f)
    }
}

impl Into<nix::sched::CpuSet> for CpuSet {
    fn into(self) -> nix::sched::CpuSet {
        let mut cpu_set = nix::sched::CpuSet::new();

        for cpu in self.cpus {
            cpu_set.set(cpu as usize).unwrap();
        }

        cpu_set
    }
}

impl From<nix::sched::CpuSet> for CpuSet {
    fn from(set: nix::sched::CpuSet) -> Self {
        let mut cpu_set = CpuSetUnchecked::empty();

        for cpu in 0 .. nix::sched::CpuSet::count() {
            if set.is_set(cpu).unwrap() {
                cpu_set = cpu_set.add_cpu(cpu as u32);
            }
        }

        cpu_set.try_into().unwrap()
    }
}

/// Get affinity to given PID
pub fn get_cpuset_to_pid(pid: Pid) -> anyhow::Result<CpuSet> {
    match nix::sched::sched_getaffinity(nix::unistd::Pid::from_raw(pid as i32)) {
        Ok(cpu_set) => {
            Ok(cpu_set.into())
        },
        Err(err) => {
            anyhow::bail!("Error in getting affinity for pid {pid}: {err}")
        },
    }
}

/// Set affinity to given PID
pub fn set_cpuset_to_pid(pid: Pid, cpu_set: &CpuSet) -> anyhow::Result<()> {
    let cpu_set_libc = cpu_set.clone().into();
    match nix::sched::sched_setaffinity(nix::unistd::Pid::from_raw(pid as i32), &cpu_set_libc) {
        Ok(()) => {
            info!("Changed CPU affinity of pid {pid} to {cpu_set:?}");
            Ok(())
        }
        Err(err) => {
            anyhow::bail!("Error in setting affinity for pid {pid}: {err}")
        },
    }
}