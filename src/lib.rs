//! Portably generate configuration for any shell.
//!
//! This crate is a Rust port of the Perl modules
//! [`Shell::Config::Generate`](https://metacpan.org/pod/Shell::Config::Generate)
//! and [`Shell::Guess`](https://metacpan.org/pod/Shell::Guess).  It lets you
//! describe environment modifications once, in a portable form, and then render
//! them as a script for `sh`, `bash`, `ksh`, `zsh`, `csh`, `tcsh`, `fish`,
//! `cmd.exe`, `command.com` or PowerShell.
//!
//! It does not touch the current process environment; it only produces text
//! that, when sourced or evaluated, will modify a shell's environment.
//!
//! # Example
//!
//! ```
//! use shell_config_generate::{Shell, ShellConfig};
//!
//! let mut config = ShellConfig::new();
//! config.comment("this is my config file");
//! config.set("FOO", "bar");
//! config.set_path("PERL5LIB", ["/foo/bar/lib/perl5", "/foo/bar/lib/perl5/perl5/site"]);
//! config.append_path("PATH", ["/foo/bar/bin", "/bar/foo/bin"]);
//!
//! print!("{}", config.generate(Shell::Bourne)?);
//! # Ok::<(), shell_config_generate::GenerateError>(())
//! ```
//!
//! produces
//!
//! ```text
//! # this is my config file
//! FOO='bar';
//! export FOO;
//! PERL5LIB='/foo/bar/lib/perl5:/foo/bar/lib/perl5/perl5/site';
//! export PERL5LIB;
//! if [ -n "$PATH" ] ; then
//!   PATH=$PATH':/foo/bar/bin:/bar/foo/bin';
//!   export PATH
//! else
//!   PATH='/foo/bar/bin:/bar/foo/bin';
//!   export PATH;
//! fi;
//! ```
//!
//! while [`Shell::C`] would produce `setenv` commands and [`Shell::Cmd`] would
//! produce `set` commands.
//!
//! # Guessing the shell
//!
//! [`Shell::running_shell`] and [`Shell::login_shell`] make a best-effort guess
//! at the shell in use, so a program can emit configuration for whichever shell
//! invoked it:
//!
//! ```no_run
//! use shell_config_generate::{Shell, ShellConfig};
//!
//! let mut config = ShellConfig::new();
//! config.set("FOO", "bar");
//! print!("{}", config.generate(Shell::running_shell())?);
//! # Ok::<(), shell_config_generate::GenerateError>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod escape;
mod shell;

pub use config::{GenerateError, ShellConfig};
pub use shell::{Shell, UnknownShell};

/// Escape a directory path (or filename) for `cmd.exe` / `command.com`,
/// returning it wrapped in double quotes.
///
/// Port of the `cmd_escape_path` function from `Shell::Config::Generate`.
///
/// ```
/// use shell_config_generate::cmd_escape_path;
/// assert_eq!(
///     cmd_escape_path(r"C:\Program Files\foo & bar"),
///     r#""C:\Program Files\foo ^& bar""#,
/// );
/// ```
pub fn cmd_escape_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 2);
    out.push('"');
    for ch in path.chars() {
        match ch {
            '%' => out.push_str("%%"),
            '&' | '^' | '|' | '<' | '>' => {
                out.push('^');
                out.push(ch);
            }
            '\n' => out.push_str("^\n\n"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Escape a list of directory paths (or filenames) for PowerShell, additionally
/// backtick-escaping spaces.
///
/// Port of the `powershell_escape_path` function from `Shell::Config::Generate`.
///
/// ```
/// use shell_config_generate::powershell_escape_path;
/// assert_eq!(
///     powershell_escape_path([r"C:\Program Files\foo", "a b"]),
///     vec![r"C:\Program` Files\foo".to_string(), "a` b".to_string()],
/// );
/// ```
pub fn powershell_escape_path<I, S>(paths: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    paths
        .into_iter()
        .map(|path| escape::value_escape_powershell(path.as_ref()).replace(' ', "` "))
        .collect()
}

/// Rewrite a list of paths to spaceless equivalents.
///
/// On `MSWin32` / Cygwin / MSYS the Perl original uses
/// `Win32::GetShortPathName` to find `8.3` aliases for any path containing
/// whitespace; everywhere else it returns its input unchanged.  This port
/// currently only implements the pass-through behavior, which is correct on
/// non-Windows platforms and a safe (if unhelpful) default on Windows.
pub fn win32_space_be_gone<I, S>(paths: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    paths.into_iter().map(Into::into).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_escape_path_specials() {
        assert_eq!(
            cmd_escape_path("a%b<c>d|e&f^g\nh"),
            "\"a%%b^<c^>d^|e^&f^^g^\n\nh\""
        );
    }

    #[test]
    fn powershell_escape_path_specials() {
        assert_eq!(
            powershell_escape_path(["a b c"]),
            vec!["a` b` c".to_string()]
        );
    }

    #[test]
    fn win32_space_be_gone_is_passthrough() {
        assert_eq!(
            win32_space_be_gone(["/a b/c", "/d/e"]),
            vec!["/a b/c".to_string(), "/d/e".to_string()]
        );
    }
}
