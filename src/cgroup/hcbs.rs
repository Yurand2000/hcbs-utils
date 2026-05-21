use std::collections::HashMap;

use crate::prelude::*;
use crate::utils::try_op_timeout;

mod uni;

pub mod prelude {
    pub use super::uni::prelude::*;
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

    pub fn set_cpu_bw_us(&mut self, runtime_us: u64, period_us: u64) -> anyhow::Result<()> {
        try_op_timeout(
            || set_cgroup_us(&self.name, Either::Left(runtime_us), period_us),
            std::time::Duration::from_millis(1000)
        )
    }

    pub fn set_cpu_max(&mut self) -> anyhow::Result<()> {
        try_op_timeout(
            || set_cgroup_us(&self.name, Either::Right(Max), 0),
            std::time::Duration::from_millis(1000)
        )
    }

    pub fn force_destroy(self) { }

    fn __force_destroy(&mut self) {
        if !cgroup_exists(&self.name) { return; }

        self.processes.clear();

        if is_pid_in_cgroup(&self.name, std::process::id()).unwrap_or(false) {
            let _ = assign_pid_to_cgroup(".", std::process::id());
        }

        if self.force_kill {
            if let Ok(pids) = cgroup_pids(&self.name) {
                pids.iter().for_each(|pid| {
                    let _ = kill_pid(*pid);
                    let _ = assign_pid_to_cgroup(".", *pid);
                });
            }
        }

        let _ = delete_cgroup(&self.name);
    }
}

impl Drop for HCBSCgroup {
    fn drop(&mut self) {
        self.__force_destroy();
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

    pub fn get_affinity(&self) -> anyhow::Result<CpuSet> {
        get_cpuset_to_pid(self.id())
    }
}

impl Drop for HCBSProcess {
    fn drop(&mut self) {
        let _ = self.set_sched_policy(SchedPolicy::other(), SchedFlags::empty());
        let _ = self.kill();
    }
}