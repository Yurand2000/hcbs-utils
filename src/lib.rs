#[allow(unused)]
#[macro_use]
extern crate log;

pub mod prelude {
    #[cfg(feature = "cgroup")]
    pub use super::cgroup::prelude::*;

    #[cfg(feature = "cpu_control")]
    pub use super::cpu_control::prelude::*;

    #[cfg(feature = "cpuset")]
    pub use super::cpuset::prelude::*;

    #[cfg(feature = "debugfs")]
    pub use super::debugfs::prelude::*;

    #[cfg(feature = "proc")]
    pub use super::proc::prelude::*;

    #[cfg(feature = "sched_policy")]
    pub use super::sched_policy::prelude::*;

    #[allow(unused)]
    pub use super::common::prelude::*;
}

#[cfg(feature = "cgroup")]
pub mod cgroup;

#[cfg(feature = "cpu_control")]
pub mod cpu_control;

#[cfg(feature = "cpuset")]
pub mod cpuset;

#[cfg(feature = "debugfs")]
pub mod debugfs;

#[cfg(feature = "proc")]
pub mod proc;

#[cfg(feature = "sched_policy")]
pub mod sched_policy;

pub mod common;

mod utils;

#[cfg(all(feature = "cgroup_is_v1", feature = "cgroup_is_v2"))]
compile_error!("feature \"cgroup_is_v1\" and feature \"cgroup_is_v2\" cannot be enabled at the same time");