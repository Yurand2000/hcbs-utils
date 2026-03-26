use crate::prelude::*;

mod uni;
mod multi;

pub mod prelude {
    pub use super::uni::prelude::*;
    pub use super::multi::prelude::*;
    pub use super::{
        HCBSCgroup,
    };
}

pub struct HCBSCgroup {
    name: String,
    force_kill: bool,
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

    pub fn set_runtime_us(&mut self, runtime_us: u64) -> anyhow::Result<()> {
        set_cgroup_runtime_us(&self.name, runtime_us)
    }

    pub fn set_period_us(&mut self, period_us: u64) -> anyhow::Result<()> {
        set_cgroup_period_us(&self.name, period_us)
    }

    pub fn set_runtime_us_multi(&mut self, runtimes_us: impl Iterator<Item = (u64, impl Iterator<Item = u32>)>) -> anyhow::Result<()> {
        set_cgroup_runtime_us_multi(&self.name, runtimes_us)
    }

    pub fn set_period_us_multi(&mut self, periods_us: impl Iterator<Item = (u64, impl Iterator<Item = u32>)>) -> anyhow::Result<()> {
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

        if self.force_kill {
            if is_pid_in_cgroup(&self.name, std::process::id())? {
                assign_pid_to_cgroup(".", std::process::id())?;
            }

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