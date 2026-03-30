use crate::prelude::*;

pub mod prelude {
    pub use super::{
        wait_pid,
        kill_pid,
    };
}

/// Wait for the given process to terminate
pub fn wait_pid(pid: Pid) -> anyhow::Result<()> {
    nix::sys::wait::waitpid(
        nix::unistd::Pid::from_raw(pid as i32),
        None
    )?;

    info!("Waited PID {pid}");

    Ok(())
}

/// Kill the given process
pub fn kill_pid(pid: Pid) -> anyhow::Result<()> {
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        nix::sys::signal::SIGKILL
    )?;

    wait_pid(pid)?;

    info!("Killed PID {pid}");

    Ok(())
}