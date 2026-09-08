//! Value-escaping helpers, one per shell family.
//!
//! Each function is a direct port of the corresponding `_value_escape_*`
//! subroutine in the Perl `Shell::Config::Generate` distribution.  The Perl
//! originals apply a sequence of independent global substitutions; because none
//! of those substitutions feed into each other, a single character-by-character
//! pass produces identical output.

/// Escape a value for use inside single quotes in csh / tcsh (`setenv`).
///
/// Port of `_value_escape_csh`: `s/([\n!])/\\$1/g` then `s/(')/'"$1"'/g`.
pub(crate) fn value_escape_csh(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\\n"),
            '!' => out.push_str("\\!"),
            '\'' => out.push_str("'\"'\"'"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escape a value for use inside single quotes in fish (`set -x`).
///
/// Port of `_value_escape_fish`: `s/([\n])/\\$1/g` then `s/(')/'"$1"'/g`.
pub(crate) fn value_escape_fish(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\\n"),
            '\'' => out.push_str("'\"'\"'"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escape a value for use inside single quotes in a Bourne-compatible shell.
///
/// Port of `_value_escape_sh`: `s/(')/'"$1"'/g`.
pub(crate) fn value_escape_sh(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

/// Escape a value for `cmd.exe` / `command.com` `set`.
///
/// Port of `_value_escape_win32`: `s/%/%%/g`, `s/([&^|<>()])/^$1/g`,
/// `s/\n/^\n\n/g`.
pub(crate) fn value_escape_win32(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '%' => out.push_str("%%"),
            '&' | '^' | '|' | '<' | '>' | '(' | ')' => {
                out.push('^');
                out.push(ch);
            }
            '\n' => out.push_str("^\n\n"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escape a value for a PowerShell double-quoted string.
///
/// Port of `_value_escape_powershell`: `s/(["'`\$#()])/`$1/g` then the control
/// character map (`` `0 `a `b `f `r `n `t ``).
pub(crate) fn value_escape_powershell(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' | '\'' | '`' | '$' | '#' | '(' | ')' => {
                out.push('`');
                out.push(ch);
            }
            '\0' => out.push_str("`0"),
            '\u{7}' => out.push_str("`a"),
            '\u{8}' => out.push_str("`b"),
            '\u{c}' => out.push_str("`f"),
            '\r' => out.push_str("`r"),
            '\n' => out.push_str("`n"),
            '\t' => out.push_str("`t"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csh() {
        assert_eq!(value_escape_csh("it's"), "it'\"'\"'s");
        assert_eq!(value_escape_csh("a!b"), "a\\!b");
        assert_eq!(value_escape_csh("a\nb"), "a\\\nb");
    }

    #[test]
    fn sh() {
        assert_eq!(value_escape_sh("it's a \"test\""), "it'\"'\"'s a \"test\"");
    }

    #[test]
    fn win32() {
        assert_eq!(
            value_escape_win32("a&b^c|d<e>f(g)h%i`j$k#l!m"),
            "a^&b^^c^|d^<e^>f^(g^)h%%i`j$k#l!m"
        );
        assert_eq!(value_escape_win32("line1\nline2"), "line1^\n\nline2");
    }

    #[test]
    fn powershell() {
        assert_eq!(
            value_escape_powershell("a&b^c|d<e>f(g)h%i`j$k#l!m"),
            "a&b^c|d<e>f`(g`)h%i``j`$k`#l!m"
        );
        assert_eq!(value_escape_powershell("a\tb\rc"), "a`tb`rc");
        assert_eq!(
            value_escape_powershell("a\u{0}b\u{7}c\u{8}d\u{c}f"),
            "a`0b`ac`bd`ff"
        );
    }
}
