# shell-config-generate

Portably generate configuration for any shell.

A Rust port of the Perl modules
[`Shell::Config::Generate`](https://metacpan.org/pod/Shell::Config::Generate)
and [`Shell::Guess`](https://metacpan.org/pod/Shell::Guess).

Describe environment modifications once, in a portable form, then render them as
a script for `sh`, `bash`, `ksh`, `zsh`, `csh`, `tcsh`, `fish`, `cmd.exe`,
`command.com` or PowerShell. Nothing in the current process environment is
touched; the crate only produces text that a shell can source or evaluate.

## Example

```rust
use shell_config_generate::{Shell, ShellConfig};

let mut config = ShellConfig::new();
config.comment("this is my config file");
config.set("FOO", "bar");
config.set_path("PERL5LIB", ["/foo/bar/lib/perl5", "/foo/bar/lib/perl5/perl5/site"]);
config.append_path("PATH", ["/foo/bar/bin", "/bar/foo/bin"]);

print!("{}", config.generate(Shell::Bourne)?);
# Ok::<(), shell_config_generate::GenerateError>(())
```

produces

```sh
# this is my config file
FOO='bar';
export FOO;
PERL5LIB='/foo/bar/lib/perl5:/foo/bar/lib/perl5/perl5/site';
export PERL5LIB;
if [ -n "$PATH" ] ; then
  PATH=$PATH':/foo/bar/bin:/bar/foo/bin';
  export PATH
else
  PATH='/foo/bar/bin:/bar/foo/bin';
  export PATH;
fi;
```

while `Shell::C` produces `setenv` commands and `Shell::Cmd` produces `set`
commands.

## Modifiers

| Method | Effect |
| --- | --- |
| `set(name, value)` | set an environment variable |
| `set_path(name, values)` | set a `PATH`-style variable, replacing any existing value |
| `append_path(name, values)` / `prepend_path(name, values)` | add to a `PATH`-style variable, creating it if needed |
| `comment(text)` / `comments(iter)` | add comment lines |
| `set_alias(alias, command)` | define an alias (a wrapper function on PowerShell) |
| `set_path_sep(sep)` | override the path separator for later path operations |
| `shebang()` / `shebang_location(path)` | prepend a `#!` line (UNIX shells only) |
| `echo_off()` / `echo_on()` | toggle a leading `@echo off` (`cmd.exe` / `command.com`) |

## Generators

- `generate(shell)` returns the rendered configuration as a `String`.
- `generate_file(shell, path)` writes it to a file.

## Guessing the shell

`Shell::running_shell()` and `Shell::login_shell()` make a best-effort,
never-failing guess at the shell in use, mirroring `Shell::Guess`. Detection is
implemented for Linux (via `/proc`) and falls back to a sensible per-platform
default elsewhere.

## Differences from the Perl module

- `win32_space_be_gone` currently only implements the non-Windows pass-through
  behavior; it does not yet call `Win32::GetShortPathName`.
- Shell detection reads `/proc` and `/etc/passwd` directly rather than using
  `getpwnam(3)` / `ps` / `dscl`.

## License

MIT
