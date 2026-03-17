pub mod prelude {
    #[cfg(feature = "proc")]
    pub use super::{
        Pid,
    };

    #[cfg(any(feature = "cpu_control", feature = "cpuset"))]
    pub use super::{
        CpuID,
    };
}

/// Type to represent PIDs of processes
#[cfg(feature = "proc")]
pub type Pid = u32;

/// Type to represent CPU ids
#[cfg(any(feature = "cpu_control", feature = "cpuset"))]
pub type CpuID = u32;