pub mod prelude {
    pub use super::{
        mount_debug_fs,
    };
}

/// Try mounting the debug filesystem
pub fn mount_debug_fs() -> anyhow::Result<(),> {
    use crate::utils::__shell;

    // Runs: mount -t debugfs none /sys/kernel/debug
    if __shell(&format!("mount | grep debugfs"))?.stdout.len() > 0 {
        debug!("DebugFS already mounted");
        return Ok(());
    }

    if !__shell(&format!("mount -t debugfs none /sys/kernel/debug"))?.status.success() {
        debug!("Error in mounting DebugFS");
        return Err(anyhow::format_err!("Error in mounting DebugFS"));
    }

    debug!("Mounted DebugFS");

    Ok(())
}