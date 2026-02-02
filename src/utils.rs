pub mod prelude {
    pub use super::{
        ROOT_CGROUP,
        Pid,
    };
}

pub const ROOT_CGROUP: &'static str = ".";

/// Type to represent PIDs of processes
pub type Pid = u32;

/// Execute the given shell command
pub fn __shell(cmd: &str) -> anyhow::Result<std::process::Output> {
    use std::process::Command;

    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|err| anyhow::format_err!("Error in executing \"sh -c {cmd}\": {err}").into())
}