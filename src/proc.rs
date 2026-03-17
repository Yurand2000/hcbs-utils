use crate::prelude::*;

pub mod prelude {
    pub use super::{
        kill_pid,
    };
}

/// Kill the given process
pub fn kill_pid(pid: Pid) -> anyhow::Result<()> {
    let pid = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), false);

    let Some(proc) = system.process(pid)
        else { warn!("Cannot kill PID {pid}: not found!"); return Ok(()); };

    let res = proc.kill_and_wait()
        .map_err(|err| anyhow::format_err!("Cannot kill PID {pid}: {err:?}"));

    info!("Killed PID {pid}");

    res?;

    Ok(())
}