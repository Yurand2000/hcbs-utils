use std::collections::HashMap;

use crate::prelude::*;

mod uni;
mod multi;

pub mod prelude {
    pub use super::uni::prelude::*;
    pub use super::multi::prelude::*;
    pub use super::{
        HCBSCgroup,
        HCBSProcess,
    };
}

pub struct HCBSCgroup {
    name: String,
    force_kill: bool,
    processes: HashMap<Pid, HCBSProcess>,
}

impl HCBSCgroup {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        if name == "." {
            anyhow::bail!("Cannot handle root cgroup");
        }

        create_cgroup(name)?;

        Ok(Self {
            name: name.to_owned(),
            force_kill: false,
            processes: HashMap::new(),
        })
    }

    pub fn with_force_kill(mut self, force_kill: bool) -> Self {
        self.force_kill = force_kill;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn destroy(mut self) -> anyhow::Result<()> {
        self.__destroy()
    }

    pub fn assign_process(&mut self, process: HCBSProcess) -> Result<&mut HCBSProcess, (HCBSProcess, anyhow::Error)> {
        let pid = process.id();

        match assign_pid_to_cgroup(&self.name, pid) {
            Ok(_) => {
                self.processes.insert(pid, process);
                Ok(self.processes.get_mut(&pid).unwrap())
            },
            Err(err) => {
                Err((process, err))
            },
        }
    }

    pub fn get_process(&self, pid: Pid) -> Option<&HCBSProcess> {
        self.processes.get(&pid)
    }

    pub fn get_process_mut(&mut self, pid: Pid) -> Option<&mut HCBSProcess> {
        self.processes.get_mut(&pid)
    }

    pub fn take_process(&mut self, pid: Pid) -> anyhow::Result<HCBSProcess> {
        if !self.processes.contains_key(&pid) {
            anyhow::bail!("No such process with PID {}", pid);
        }

        assign_pid_to_cgroup(&self.name, pid)
            .map(|_| self.processes.remove(&pid).unwrap())
    }

    pub fn set_runtime_us(&mut self, runtime_us: u64) -> anyhow::Result<()> {
        set_cgroup_runtime_us(&self.name, runtime_us)
    }

    pub fn set_period_us(&mut self, period_us: u64) -> anyhow::Result<()> {
        set_cgroup_period_us(&self.name, period_us)
    }

    pub fn set_runtime_us_multi<I, J>(&mut self, runtimes_us: I) -> anyhow::Result<()>
        where I: IntoIterator<Item = (u64, J)>, J: IntoIterator<Item = CpuID>
    {
        set_cgroup_runtime_us_multi(&self.name, runtimes_us)
    }

    pub fn set_period_us_multi<I, J>(&mut self, periods_us: I) -> anyhow::Result<()>
        where I: IntoIterator<Item = (u64, J)>, J: IntoIterator<Item = CpuID>
    {
        set_cgroup_period_us_multi(&self.name, periods_us)
    }

    pub fn set_runtime_us_multi_str(&mut self, runtimes_us: &str) -> anyhow::Result<()> {
        set_cgroup_runtime_us_multi_str(&self.name, runtimes_us)
    }

    pub fn set_period_us_multi_str(&mut self, periods_us: &str) -> anyhow::Result<()> {
        set_cgroup_period_us_multi_str(&self.name, periods_us)
    }

    fn __destroy(&mut self) -> anyhow::Result<()> {
        if !cgroup_exists(&self.name) { return Ok(()); }

        self.processes.clear();

        if is_pid_in_cgroup(&self.name, std::process::id())? {
            assign_pid_to_cgroup(".", std::process::id())?;
        }

        if self.force_kill {
            cgroup_pids(&self.name)?.iter()
                .try_for_each(|pid| {
                    kill_pid(*pid)?;
                    assign_pid_to_cgroup(".", *pid)
                })?;
        }

        delete_cgroup(&self.name)
    }
}

impl Drop for HCBSCgroup {
    fn drop(&mut self) {
        let _ = self.__destroy();
    }
}

pub enum HCBSProcess {
    Child(std::process::Child),
    SelfProc,
}

impl From<std::process::Child> for HCBSProcess {
    fn from(proc: std::process::Child) -> Self {
        Self::Child(proc)
    }
}

impl HCBSProcess {
    pub fn id(&self) -> Pid {
        match self {
            HCBSProcess::Child(child) => child.id(),
            HCBSProcess::SelfProc => std::process::id(),
        }
    }

    pub fn wait(&mut self) -> anyhow::Result<()> {
        match self {
            HCBSProcess::Child(child) => wait_pid(child.id()),
            HCBSProcess::SelfProc => anyhow::bail!("Cannot wait Self Process"),
        }
    }

    pub fn kill(&mut self) -> anyhow::Result<()> {
        match self {
            HCBSProcess::Child(child) => kill_pid(child.id()),
            HCBSProcess::SelfProc => anyhow::bail!("Cannot kill Self Process"),
        }
    }

    pub fn set_sched_policy(&mut self, policy: SchedPolicy, flags: SchedFlags) -> Result<(), SetSchedError> {
        set_sched_policy(self.id(), policy, flags)
    }

    pub fn get_sched_policy(&self) -> Result<(SchedPolicy, SchedFlags), GetSchedError> {
        get_sched_policy(self.id())
    }

    pub fn set_affinity(&mut self, affinity: CpuSet) -> anyhow::Result<()> {
        set_cpuset_to_pid(self.id(), &affinity)
    }

    pub fn get_affinity(&mut self) -> anyhow::Result<CpuSet> {
        get_cpuset_to_pid(self.id())
    }
}

impl Drop for HCBSProcess {
    fn drop(&mut self) {
        let _ = self.set_sched_policy(SchedPolicy::other(), SchedFlags::empty());
        let _ = self.kill();
    }
}