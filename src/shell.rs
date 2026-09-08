//! Shell identification, a port of the Perl `Shell::Guess` module.

use std::fmt;
use std::str::FromStr;

/// A kind of shell, together with the family predicates used to decide how to
/// render configuration for it.
///
/// This mirrors the instances produced by `Shell::Guess` (`bourne_shell`,
/// `c_shell`, `cmd_shell`, ...) and the `is_*` interrogation methods.
///
/// ```
/// use shell_config_generate::Shell;
///
/// assert!(Shell::Bash.is_bourne());
/// assert!(Shell::Bash.is_unix());
/// assert!(!Shell::Bash.is_win32());
/// assert_eq!(Shell::Bash.name(), "bash");
/// assert_eq!(Shell::Bash.default_location(), Some("/bin/bash"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shell {
    /// bash (`is_bash`, `is_bourne`, `is_unix`).
    Bash,
    /// The original Bourne shell, `/bin/sh` (`is_bourne`, `is_unix`).
    Bourne,
    /// csh (`is_c`, `is_unix`).
    C,
    /// The Windows NT command interpreter, `cmd.exe` (`is_cmd`, `is_win32`).
    Cmd,
    /// The Windows 9x / MS-DOS command interpreter, `command.com`
    /// (`is_command`, `is_win32`).
    Command,
    /// OpenVMS DCL (`is_dcl`, `is_vms`).
    Dcl,
    /// fish (`is_fish`, `is_unix`).
    Fish,
    /// The Korn shell, ksh (`is_korn`, `is_bourne`, `is_unix`).
    Korn,
    /// Microsoft PowerShell, `powershell.exe` or `pwsh` (`is_power`, `is_win32`).
    Power,
    /// tcsh (`is_tc`, `is_c`, `is_unix`).
    Tc,
    /// zsh (`is_z`, `is_bourne`, `is_unix`).
    Z,
}

impl Shell {
    /// The short name of the shell, matching `Shell::Guess`'s `name` method and
    /// the `<name>_shell` constructors (`"bourne"`, `"c"`, `"tc"`, ...).
    pub fn name(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Bourne => "bourne",
            Shell::C => "c",
            Shell::Cmd => "cmd",
            Shell::Command => "command",
            Shell::Dcl => "dcl",
            Shell::Fish => "fish",
            Shell::Korn => "korn",
            Shell::Power => "power",
            Shell::Tc => "tc",
            Shell::Z => "z",
        }
    }

    /// The usual filesystem location for this shell, e.g. `/bin/sh` for the
    /// Bourne shell.  Not every shell has one (`fish`, `power` and `dcl` return
    /// `None`, matching `Shell::Guess`, where `default_location` is undefined).
    pub fn default_location(self) -> Option<&'static str> {
        Some(match self {
            Shell::Bash => "/bin/bash",
            Shell::Bourne => "/bin/sh",
            Shell::C => "/bin/csh",
            Shell::Cmd => "C:\\Windows\\system32\\cmd.exe",
            Shell::Command => "C:\\Windows\\system32\\command.com",
            Shell::Korn => "/bin/ksh",
            Shell::Tc => "/bin/tcsh",
            Shell::Z => "/bin/zsh",
            Shell::Dcl | Shell::Fish | Shell::Power => return None,
        })
    }

    /// Look up a [`Shell`] by its short [`name`](Shell::name).
    ///
    /// ```
    /// use shell_config_generate::Shell;
    /// assert_eq!(Shell::from_name("tc"), Some(Shell::Tc));
    /// assert_eq!(Shell::from_name("nope"), None);
    /// ```
    pub fn from_name(name: &str) -> Option<Shell> {
        Some(match name {
            "bash" => Shell::Bash,
            "bourne" => Shell::Bourne,
            "c" => Shell::C,
            "cmd" => Shell::Cmd,
            "command" => Shell::Command,
            "dcl" => Shell::Dcl,
            "fish" => Shell::Fish,
            "korn" => Shell::Korn,
            "power" => Shell::Power,
            "tc" => Shell::Tc,
            "z" => Shell::Z,
            _ => return None,
        })
    }

    /// True for bash.
    pub fn is_bash(self) -> bool {
        self == Shell::Bash
    }

    /// True for the Bourne shell and every shell that understands Bourne syntax
    /// (bash, ksh, zsh).
    pub fn is_bourne(self) -> bool {
        matches!(self, Shell::Bash | Shell::Bourne | Shell::Korn | Shell::Z)
    }

    /// True for csh and every shell that understands csh syntax (tcsh).
    pub fn is_c(self) -> bool {
        matches!(self, Shell::C | Shell::Tc)
    }

    /// True for the Windows NT `cmd.exe` shell.
    pub fn is_cmd(self) -> bool {
        self == Shell::Cmd
    }

    /// True for the Windows 9x / MS-DOS `command.com` shell.
    pub fn is_command(self) -> bool {
        self == Shell::Command
    }

    /// True for the OpenVMS DCL shell.
    pub fn is_dcl(self) -> bool {
        self == Shell::Dcl
    }

    /// True for the fish shell.
    pub fn is_fish(self) -> bool {
        self == Shell::Fish
    }

    /// True for the Korn shell.
    pub fn is_korn(self) -> bool {
        self == Shell::Korn
    }

    /// True for PowerShell.
    pub fn is_power(self) -> bool {
        self == Shell::Power
    }

    /// True for tcsh.
    pub fn is_tc(self) -> bool {
        self == Shell::Tc
    }

    /// True for shells traditionally found on UNIX (bourne, bash, csh, ...).
    pub fn is_unix(self) -> bool {
        matches!(
            self,
            Shell::Bash
                | Shell::Bourne
                | Shell::C
                | Shell::Fish
                | Shell::Korn
                | Shell::Tc
                | Shell::Z
        )
    }

    /// True for shells traditionally found on OpenVMS (dcl).
    pub fn is_vms(self) -> bool {
        self == Shell::Dcl
    }

    /// True for shells traditionally found on Windows (`command.com`, `cmd.exe`,
    /// PowerShell).
    pub fn is_win32(self) -> bool {
        matches!(self, Shell::Cmd | Shell::Command | Shell::Power)
    }

    /// True for zsh.
    pub fn is_z(self) -> bool {
        self == Shell::Z
    }

    /// Make an educated guess about the shell that invoked the current process,
    /// a port of `Shell::Guess->running_shell`.
    ///
    /// On Linux this inspects `/proc/<ppid>/comm` and `/proc/<ppid>/cmdline`.
    /// When the running shell cannot be determined it falls back to
    /// [`login_shell`](Shell::login_shell).  Like the Perl original it is
    /// best-effort and never fails.
    pub fn running_shell() -> Shell {
        running_shell_opt().unwrap_or_else(Shell::login_shell)
    }

    /// Make an educated guess about the current user's login shell, a port of
    /// `Shell::Guess->login_shell`.
    ///
    /// The user name is taken from `$USER`, `$USERNAME` or `$LOGNAME`.  When no
    /// shell can be guessed a reasonable per-platform fallback is used (Bourne
    /// on UNIX, `cmd.exe` on Windows).
    pub fn login_shell() -> Shell {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .or_else(|_| std::env::var("LOGNAME"))
            .ok();
        match user {
            Some(user) => Shell::login_shell_for(&user),
            None => login_shell_fallback(),
        }
    }

    /// Make an educated guess about a specific user's login shell, a port of
    /// `Shell::Guess->login_shell($username)`.
    ///
    /// On UNIX this reads the user's shell from `/etc/passwd`.
    pub fn login_shell_for(username: &str) -> Shell {
        #[cfg(unix)]
        {
            if let Ok(passwd) = std::fs::read_to_string("/etc/passwd") {
                for line in passwd.lines() {
                    let mut fields = line.split(':');
                    if fields.next() == Some(username) {
                        if let Some(shell) = fields.next_back() {
                            if let Some(shell) = unixy_shell(shell) {
                                return shell;
                            }
                        }
                    }
                }
            }
        }
        let _ = username;
        login_shell_fallback()
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Shell {
    type Err = UnknownShell;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Shell::from_name(s).ok_or_else(|| UnknownShell(s.to_owned()))
    }
}

/// Error returned when a shell name is not recognised (see [`Shell::from_str`]).
///
/// Its `Display` matches the Perl `Shell::Config::Generate` message
/// `unknown shell type: <name>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownShell(pub String);

impl fmt::Display for UnknownShell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown shell type: {}", self.0)
    }
}

impl std::error::Error for UnknownShell {}

/// Map a command name or path to a shell by suffix, a port of
/// `Shell::Guess::_unixy_shells`.  Order matters: `tcsh` before `csh`, and both
/// before the bare `sh` catch-all.
fn unixy_shell(command: &str) -> Option<Shell> {
    let command = command.trim();
    if command.ends_with("tcsh") {
        Some(Shell::Tc)
    } else if command.ends_with("csh") {
        Some(Shell::C)
    } else if command.ends_with("ksh") {
        Some(Shell::Korn)
    } else if command.ends_with("bash") {
        Some(Shell::Bash)
    } else if command.ends_with("zsh") {
        Some(Shell::Z)
    } else if command.ends_with("fish") {
        Some(Shell::Fish)
    } else if command.ends_with("pwsh") {
        Some(Shell::Power)
    } else if command.ends_with("sh") {
        Some(Shell::Bourne)
    } else {
        None
    }
}

fn running_shell_opt() -> Option<Shell> {
    #[cfg(unix)]
    {
        let ppid = read_ppid()?;
        for path in [
            format!("/proc/{ppid}/comm"),
            format!("/proc/{ppid}/cmdline"),
        ] {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                // Both files may carry NUL separators; the command is the first
                // field.  `comm` also has a trailing newline, which `unixy_shell`
                // trims.
                let command = contents.split('\0').next().unwrap_or("");
                if let Some(shell) = unixy_shell(command) {
                    return Some(shell);
                }
            }
        }
        None
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(unix)]
fn read_ppid() -> Option<u32> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("PPid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

fn login_shell_fallback() -> Shell {
    #[cfg(unix)]
    {
        if let Ok(shell) = std::env::var("SHELL") {
            if let Some(shell) = unixy_shell(&shell) {
                return shell;
            }
        }
        Shell::Bourne
    }
    #[cfg(not(unix))]
    {
        #[cfg(windows)]
        {
            Shell::Cmd
        }
        #[cfg(not(windows))]
        {
            Shell::Bourne
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicates() {
        for (shell, name, unix, win32) in [
            (Shell::Bash, "bash", true, false),
            (Shell::Bourne, "bourne", true, false),
            (Shell::C, "c", true, false),
            (Shell::Cmd, "cmd", false, true),
            (Shell::Command, "command", false, true),
            (Shell::Dcl, "dcl", false, false),
            (Shell::Fish, "fish", true, false),
            (Shell::Korn, "korn", true, false),
            (Shell::Power, "power", false, true),
            (Shell::Tc, "tc", true, false),
            (Shell::Z, "z", true, false),
        ] {
            assert_eq!(shell.name(), name);
            assert_eq!(shell.is_unix(), unix, "{name} is_unix");
            assert_eq!(shell.is_win32(), win32, "{name} is_win32");
            assert_eq!(Shell::from_name(name), Some(shell));
            assert_eq!(name.parse::<Shell>().unwrap(), shell);
        }
    }

    #[test]
    fn families() {
        assert!(Shell::Bash.is_bourne() && Shell::Korn.is_bourne() && Shell::Z.is_bourne());
        assert!(!Shell::C.is_bourne());
        assert!(Shell::Tc.is_c() && Shell::C.is_c());
        assert!(Shell::Dcl.is_vms());
        assert!(Shell::Power.is_win32());
    }

    #[test]
    fn unknown_shell() {
        let err = "zwsh".parse::<Shell>().unwrap_err();
        assert_eq!(err.to_string(), "unknown shell type: zwsh");
    }

    #[test]
    fn unixy_shell_suffixes() {
        assert_eq!(unixy_shell("/bin/tcsh"), Some(Shell::Tc));
        assert_eq!(unixy_shell("/bin/csh"), Some(Shell::C));
        assert_eq!(unixy_shell("-bash"), Some(Shell::Bash));
        assert_eq!(unixy_shell("/usr/bin/zsh\n"), Some(Shell::Z));
        assert_eq!(unixy_shell("/usr/local/bin/fish"), Some(Shell::Fish));
        assert_eq!(unixy_shell("pwsh"), Some(Shell::Power));
        assert_eq!(unixy_shell("/bin/sh"), Some(Shell::Bourne));
        assert_eq!(unixy_shell("/usr/bin/perl"), None);
    }

    #[test]
    fn guessers_never_panic() {
        // Best-effort, environment dependent: assert only that they return.
        let _ = Shell::running_shell();
        let _ = Shell::login_shell();
    }
}
