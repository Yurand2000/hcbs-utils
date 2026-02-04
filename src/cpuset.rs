use crate::prelude::*;

pub mod prelude {
    pub use super::{
        CpuSet,
        CpuSetUnchecked,
        CpuSetBuildError,
        set_cpuset_to_pid
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

    /// Return the number of CPUs in the set
    pub fn num_cpus(&self) -> usize {
        self.cpus.len()
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

    /// Get the number of CPUs in the set
    pub fn num_cpus(&self) -> usize {
        self.cpus.len()
    }
}

impl std::str::FromStr for CpuSet {
    type Err = CpuSetBuildError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match CpuSetUnchecked::from_str(s) {
            Ok(cpus) => cpus.into(),
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

impl Into<Result<CpuSet, CpuSetBuildError>> for CpuSetUnchecked {
    fn into(self) -> Result<CpuSet, CpuSetBuildError> {
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

impl From<&CpuSet> for scheduler::CpuSet {
    fn from(cpuset: &CpuSet) -> Self {
        let mut out = scheduler::CpuSet::new(0);
        cpuset.cpus.iter()
            .for_each(|cpu| out.set(*cpu as usize));

        out
    }
}

/// Set affinity to given PID
pub fn set_cpuset_to_pid(pid: Pid, cpu_set: &CpuSet) -> Result<(), Box<dyn std::error::Error>> {
    scheduler::set_affinity(pid as i32, cpu_set.into())
        .map_err(|_| format!("Error in setting affinity for pid {pid}"))?;

    info!("Changed CPU affinity of pid {pid} to {cpu_set:?}");

    Ok(())
}