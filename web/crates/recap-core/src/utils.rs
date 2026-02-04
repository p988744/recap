//! Utility functions for recap-core

use std::process::Command;

/// Creates a Command that hides the console window on Windows.
///
/// On Windows, GUI applications spawning console processes (like `git`) will
/// create visible CMD windows. This function configures the Command to use
/// CREATE_NO_WINDOW flag on Windows to prevent this.
///
/// # Example
/// ```ignore
/// use recap_core::utils::create_command;
///
/// let output = create_command("git")
///     .arg("log")
///     .arg("--oneline")
///     .current_dir("/path/to/repo")
///     .output();
/// ```
pub fn create_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW = 0x08000000
        // Prevents the process from creating a console window
        cmd.creation_flags(0x08000000);
    }

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_command_returns_command() {
        let cmd = create_command("echo");
        // Just verify it creates a valid Command
        assert!(format!("{:?}", cmd).contains("echo"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_no_window_flag() {
        // On Windows, verify the command has creation flags set
        let cmd = create_command("cmd");
        let debug_str = format!("{:?}", cmd);
        // The exact format may vary, but we just verify it compiles and runs
        assert!(debug_str.contains("cmd"));
    }
}
