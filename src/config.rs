//! Portable shell configuration, a port of the Perl `Shell::Config::Generate`
//! module.

use std::fmt;
use std::fmt::Write as _;
use std::path::Path;

use crate::escape::{
    value_escape_csh, value_escape_fish, value_escape_powershell, value_escape_sh,
    value_escape_win32,
};
use crate::shell::Shell;

/// A single deferred modification, stored until [`ShellConfig::generate`] turns
/// it into shell-specific text.
#[derive(Debug, Clone)]
enum Command {
    Set { name: String, value: String },
    SetPath { name: String, values: Vec<String> },
    AppendPath { name: String, values: Vec<String> },
    PrependPath { name: String, values: Vec<String> },
    Comment(String),
    Alias { alias: String, command: String },
    SetPathSep(String),
}

#[derive(Debug, Clone)]
enum Shebang {
    /// `shebang()` with no argument: use the shell's default location.
    Default,
    /// `shebang($location)`: use an explicit interpreter path.
    Location(String),
}

/// An accumulator for environment modifications that can be rendered for any
/// supported shell.
///
/// The workflow mirrors the Perl module: apply *modifiers* ([`set`](Self::set),
/// [`append_path`](Self::append_path), [`comment`](Self::comment), ...) in a
/// portable form, then call a *generator* ([`generate`](Self::generate) /
/// [`generate_file`](Self::generate_file)) once per target shell.
///
/// ```
/// use shell_config_generate::{Shell, ShellConfig};
///
/// let mut config = ShellConfig::new();
/// config.comment("this is my config file");
/// config.set("FOO", "bar");
/// config.set_path("PERL5LIB", ["/foo/bar/lib/perl5", "/foo/bar/lib/perl5/perl5/site"]);
/// config.append_path("PATH", ["/foo/bar/bin", "/bar/foo/bin"]);
///
/// let sh = config.generate(Shell::Bourne)?;
/// assert!(sh.starts_with("# this is my config file\nFOO='bar';\nexport FOO;\n"));
///
/// let csh = config.generate(Shell::C)?;
/// assert!(csh.contains("setenv FOO 'bar';\n"));
/// # Ok::<(), shell_config_generate::GenerateError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct ShellConfig {
    commands: Vec<Command>,
    shebang: Option<Shebang>,
    echo_off: bool,
}

impl ShellConfig {
    /// Create an empty configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an environment variable.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.commands.push(Command::Set {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Set an environment variable stored in the platform's `PATH` format (a
    /// list joined by `:` on UNIX or `;` on Windows).  Replaces any existing
    /// value.
    pub fn set_path<I, S>(&mut self, name: impl Into<String>, values: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.commands.push(Command::SetPath {
            name: name.into(),
            values: values.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// Append one or more entries to an environment variable stored in `PATH`
    /// format, creating it if it does not yet exist.
    ///
    /// As in the Perl module, an empty list is a no-op.
    pub fn append_path<I, S>(&mut self, name: impl Into<String>, values: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let values: Vec<String> = values.into_iter().map(Into::into).collect();
        if !values.is_empty() {
            self.commands.push(Command::AppendPath {
                name: name.into(),
                values,
            });
        }
        self
    }

    /// Prepend one or more entries to an environment variable stored in `PATH`
    /// format, creating it if it does not yet exist.
    ///
    /// As in the Perl module, an empty list is a no-op.
    pub fn prepend_path<I, S>(&mut self, name: impl Into<String>, values: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let values: Vec<String> = values.into_iter().map(Into::into).collect();
        if !values.is_empty() {
            self.commands.push(Command::PrependPath {
                name: name.into(),
                values,
            });
        }
        self
    }

    /// Add a comment.  Embedded newlines produce multiple comment lines.
    pub fn comment(&mut self, comment: impl Into<String>) -> &mut Self {
        self.commands.push(Command::Comment(comment.into()));
        self
    }

    /// Add several comments at once (equivalent to calling [`comment`](Self::comment)
    /// for each element).
    pub fn comments<I, S>(&mut self, comments: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for comment in comments {
            self.commands.push(Command::Comment(comment.into()));
        }
        self
    }

    /// Emit a `#!` line at the top of the configuration, using the target
    /// shell's [default location](Shell::default_location).  Ignored for
    /// non-UNIX shells.
    pub fn shebang(&mut self) -> &mut Self {
        self.shebang = Some(Shebang::Default);
        self
    }

    /// Emit a `#!` line at the top of the configuration using an explicit
    /// interpreter location.  Ignored for non-UNIX shells.
    pub fn shebang_location(&mut self, location: impl Into<String>) -> &mut Self {
        self.shebang = Some(Shebang::Location(location.into()));
        self
    }

    /// For `cmd.exe` / `command.com`, emit `@echo off` as the first line.
    pub fn echo_off(&mut self) -> &mut Self {
        self.echo_off = true;
        self
    }

    /// Undo [`echo_off`](Self::echo_off) (the default).
    pub fn echo_on(&mut self) -> &mut Self {
        self.echo_off = false;
        self
    }

    /// Define an alias.
    ///
    /// Caveat (from the Perl docs): some shells such as the original Bourne
    /// shell do not support aliases; this still emits one, since `/bin/sh` is
    /// often a more capable shell.  For PowerShell a wrapper function is emitted
    /// so that arguments are forwarded.
    pub fn set_alias(&mut self, alias: impl Into<String>, command: impl Into<String>) -> &mut Self {
        self.commands.push(Command::Alias {
            alias: alias.into(),
            command: command.into(),
        });
        self
    }

    /// Use `sep` as the path separator for subsequent path operations instead of
    /// the shell default.
    pub fn set_path_sep(&mut self, sep: impl Into<String>) -> &mut Self {
        self.commands.push(Command::SetPathSep(sep.into()));
        self
    }

    /// Render the configuration for `shell`.
    ///
    /// # Errors
    ///
    /// Returns [`GenerateError::Unsupported`] when the configuration uses an
    /// operation the target shell has no rendering for (for example any
    /// operation with [`Shell::Dcl`], or a comment with a shell that is neither
    /// UNIX-like nor PowerShell).
    pub fn generate(&self, shell: Shell) -> Result<String, GenerateError> {
        let mut buffer = String::new();
        let mut sep: String = if shell.is_win32() {
            ";".into()
        } else {
            ":".into()
        };

        if let Some(shebang) = &self.shebang {
            if shell.is_unix() {
                let location = match shebang {
                    Shebang::Location(location) => location.as_str(),
                    Shebang::Default => shell.default_location().unwrap_or(""),
                };
                let _ = writeln!(buffer, "#!{location}");
            }
        }

        if self.echo_off && (shell.is_cmd() || shell.is_command()) {
            buffer.push_str("@echo off\n");
        }

        for command in &self.commands {
            match command {
                Command::SetPathSep(new_sep) => sep = new_sep.clone(),
                Command::Set { name, value } => emit_set(&mut buffer, shell, name, value)?,
                Command::SetPath { name, values } => {
                    emit_set(&mut buffer, shell, name, &values.join(&sep))?
                }
                Command::AppendPath { name, values } => {
                    emit_path(&mut buffer, shell, &sep, name, values, PathOp::Append)?
                }
                Command::PrependPath { name, values } => {
                    emit_path(&mut buffer, shell, &sep, name, values, PathOp::Prepend)?
                }
                Command::Comment(text) => emit_comment(&mut buffer, shell, text)?,
                Command::Alias { alias, command } => {
                    emit_alias(&mut buffer, shell, alias, command)?
                }
            }
        }

        Ok(buffer)
    }

    /// Render the configuration for `shell` and write it to `path`.
    ///
    /// # Errors
    ///
    /// Returns [`GenerateError::Unsupported`] as [`generate`](Self::generate)
    /// does, or [`GenerateError::Io`] if the file cannot be written.
    pub fn generate_file(&self, shell: Shell, path: impl AsRef<Path>) -> Result<(), GenerateError> {
        let contents = self.generate(shell)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathOp {
    Append,
    Prepend,
}

impl PathOp {
    fn is_prepend(self) -> bool {
        self == PathOp::Prepend
    }
}

fn emit_set(
    buffer: &mut String,
    shell: Shell,
    name: &str,
    value: &str,
) -> Result<(), GenerateError> {
    if shell.is_c() {
        let value = value_escape_csh(value);
        let _ = writeln!(buffer, "setenv {name} '{value}';");
    } else if shell.is_fish() {
        let value = value_escape_fish(value);
        let _ = writeln!(buffer, "set -x {name} '{value}';");
    } else if shell.is_bourne() {
        let value = value_escape_sh(value);
        let _ = writeln!(buffer, "{name}='{value}';");
        let _ = writeln!(buffer, "export {name};");
    } else if shell.is_cmd() || shell.is_command() {
        let value = value_escape_win32(value);
        let _ = writeln!(buffer, "set {name}={value}");
    } else if shell.is_power() {
        let value = value_escape_powershell(value);
        let _ = writeln!(buffer, "$env:{name} = \"{value}\"");
    } else {
        return Err(GenerateError::unsupported("set", shell));
    }
    Ok(())
}

fn emit_path(
    buffer: &mut String,
    shell: Shell,
    sep: &str,
    name: &str,
    values: &[String],
    op: PathOp,
) -> Result<(), GenerateError> {
    if shell.is_c() {
        let value = join_escaped(values, value_escape_csh, sep);
        let _ = write!(
            buffer,
            "test \"$?{name}\" = 0 && setenv {name} '{value}' || "
        );
        if op.is_prepend() {
            let _ = write!(buffer, "setenv {name} '{value}{sep}'\"${name}\"");
        } else {
            let _ = write!(buffer, "setenv {name} \"${name}\"'{sep}{value}'");
        }
        buffer.push_str(";\n");
    } else if shell.is_bourne() {
        let value = join_escaped(values, value_escape_sh, sep);
        let _ = writeln!(buffer, "if [ -n \"${name}\" ] ; then");
        if op.is_prepend() {
            let _ = writeln!(buffer, "  {name}='{value}{sep}'${name};");
            let _ = writeln!(buffer, "  export {name};");
        } else {
            let _ = writeln!(buffer, "  {name}=${name}'{sep}{value}';");
            let _ = writeln!(buffer, "  export {name}");
        }
        let _ = writeln!(buffer, "else");
        let _ = writeln!(buffer, "  {name}='{value}';");
        let _ = writeln!(buffer, "  export {name};");
        let _ = writeln!(buffer, "fi;");
    } else if shell.is_fish() {
        let value = join_escaped(values, value_escape_fish, " ");
        let _ = write!(
            buffer,
            "if [ \"${name}\" == \"\" ]; set -x {name} {value}; else; "
        );
        if op.is_prepend() {
            let _ = write!(buffer, "set -x {name} {value} ${name};");
        } else {
            let _ = write!(buffer, "set -x {name} ${name} {value};");
        }
        buffer.push_str("end\n");
    } else if shell.is_cmd() || shell.is_command() || shell.is_power() {
        if shell.is_power() {
            let value = join_escaped(values, value_escape_powershell, sep);
            let _ = write!(buffer, "if($env:{name}) {{ ");
            if op.is_prepend() {
                let _ = write!(buffer, "$env:{name} = \"{value}{sep}\" + $env:{name}");
            } else {
                let _ = write!(buffer, "$env:{name} = $env:{name} + \"{sep}{value}\"");
            }
            let _ = writeln!(buffer, " }} else {{ $env:{name} = \"{value}\" }}");
        } else {
            let value = join_escaped(values, value_escape_win32, sep);
            let _ = write!(buffer, "if defined {name} (set ");
            if op.is_prepend() {
                let _ = write!(buffer, "{name}={value}{sep}%{name}%");
            } else {
                let _ = write!(buffer, "{name}=%{name}%{sep}{value}");
            }
            let _ = writeln!(buffer, ") else (set {name}={value})");
        }
    } else {
        // The Perl croak text says "append_path" for both directions.
        return Err(GenerateError::unsupported("append_path", shell));
    }
    Ok(())
}

fn emit_comment(buffer: &mut String, shell: Shell, text: &str) -> Result<(), GenerateError> {
    let prefix = if shell.is_unix() || shell.is_power() {
        "# "
    } else if shell.is_cmd() || shell.is_command() {
        "rem "
    } else {
        return Err(GenerateError::unsupported("comment", shell));
    };
    for line in split_newlines(text) {
        let _ = writeln!(buffer, "{prefix}{line}");
    }
    Ok(())
}

fn emit_alias(
    buffer: &mut String,
    shell: Shell,
    alias: &str,
    command: &str,
) -> Result<(), GenerateError> {
    if shell.is_bourne() {
        let _ = writeln!(buffer, "alias {alias}=\"{command}\";");
    } else if shell.is_c() {
        let _ = writeln!(buffer, "alias {alias} {command};");
    } else if shell.is_cmd() || shell.is_command() {
        let _ = writeln!(buffer, "DOSKEY {alias}={command} $*");
    } else if shell.is_power() {
        let _ = writeln!(
            buffer,
            "function {alias} {{ {} $args }}",
            value_escape_powershell(command)
        );
    } else if shell.is_fish() {
        let _ = writeln!(buffer, "alias {alias} '{command}';");
    } else {
        return Err(GenerateError::unsupported("alias", shell));
    }
    Ok(())
}

fn join_escaped(values: &[String], escape: fn(&str) -> String, sep: &str) -> String {
    values
        .iter()
        .map(|value| escape(value))
        .collect::<Vec<_>>()
        .join(sep)
}

/// Split on `\n` with Perl `split /\n/` semantics: trailing empty fields are
/// dropped, so a trailing newline does not produce an extra empty line.
fn split_newlines(text: &str) -> impl Iterator<Item = &str> {
    let mut parts: Vec<&str> = text.split('\n').collect();
    while parts.last() == Some(&"") {
        parts.pop();
    }
    parts.into_iter()
}

/// The error type for [`ShellConfig::generate`] and
/// [`ShellConfig::generate_file`].
#[derive(Debug)]
#[non_exhaustive]
pub enum GenerateError {
    /// The target shell has no rendering for one of the requested operations.
    ///
    /// `Display` matches the Perl croak text, e.g.
    /// `don't know how to "set" with dcl`.
    Unsupported {
        /// The operation name (`"set"`, `"append_path"`, `"comment"`, `"alias"`).
        operation: &'static str,
        /// The shell that does not support it.
        shell: Shell,
    },
    /// [`ShellConfig::generate_file`] failed to write the output file.
    Io(std::io::Error),
}

impl GenerateError {
    fn unsupported(operation: &'static str, shell: Shell) -> Self {
        GenerateError::Unsupported { operation, shell }
    }
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenerateError::Unsupported { operation, shell } => {
                write!(f, "don't know how to \"{operation}\" with {}", shell.name())
            }
            GenerateError::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for GenerateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GenerateError::Io(error) => Some(error),
            GenerateError::Unsupported { .. } => None,
        }
    }
}

impl From<std::io::Error> for GenerateError {
    fn from(error: std::io::Error) -> Self {
        GenerateError::Io(error)
    }
}
