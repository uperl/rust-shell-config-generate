//! Output parity tests.
//!
//! Every `expected` string in this file was captured from the Perl
//! `Shell::Config::Generate` 0.34 / `Shell::Guess` 0.10 distribution running the
//! equivalent program.

use shell_config_generate::{GenerateError, Shell, ShellConfig};

fn synopsis() -> ShellConfig {
    let mut config = ShellConfig::new();
    config.comment("this is my config file");
    config.set("FOO", "bar");
    config.set_path(
        "PERL5LIB",
        ["/foo/bar/lib/perl5", "/foo/bar/lib/perl5/perl5/site"],
    );
    config.append_path("PATH", ["/foo/bar/bin", "/bar/foo/bin"]);
    config
}

#[test]
fn synopsis_bourne_family() {
    let expected = "\
# this is my config file
FOO='bar';
export FOO;
PERL5LIB='/foo/bar/lib/perl5:/foo/bar/lib/perl5/perl5/site';
export PERL5LIB;
if [ -n \"$PATH\" ] ; then
  PATH=$PATH':/foo/bar/bin:/bar/foo/bin';
  export PATH
else
  PATH='/foo/bar/bin:/bar/foo/bin';
  export PATH;
fi;
";
    for shell in [Shell::Bourne, Shell::Bash, Shell::Korn, Shell::Z] {
        assert_eq!(synopsis().generate(shell).unwrap(), expected, "{shell}");
    }
}

#[test]
fn synopsis_c_family() {
    let expected = "\
# this is my config file
setenv FOO 'bar';
setenv PERL5LIB '/foo/bar/lib/perl5:/foo/bar/lib/perl5/perl5/site';
test \"$?PATH\" = 0 && setenv PATH '/foo/bar/bin:/bar/foo/bin' || setenv PATH \"$PATH\"':/foo/bar/bin:/bar/foo/bin';
";
    for shell in [Shell::C, Shell::Tc] {
        assert_eq!(synopsis().generate(shell).unwrap(), expected, "{shell}");
    }
}

#[test]
fn synopsis_fish() {
    let expected = "\
# this is my config file
set -x FOO 'bar';
set -x PERL5LIB '/foo/bar/lib/perl5:/foo/bar/lib/perl5/perl5/site';
if [ \"$PATH\" == \"\" ]; set -x PATH /foo/bar/bin /bar/foo/bin; else; set -x PATH $PATH /foo/bar/bin /bar/foo/bin;end
";
    assert_eq!(synopsis().generate(Shell::Fish).unwrap(), expected);
}

#[test]
fn synopsis_win32_cmd() {
    let expected = "\
rem this is my config file
set FOO=bar
set PERL5LIB=/foo/bar/lib/perl5;/foo/bar/lib/perl5/perl5/site
if defined PATH (set PATH=%PATH%;/foo/bar/bin;/bar/foo/bin) else (set PATH=/foo/bar/bin;/bar/foo/bin)
";
    for shell in [Shell::Cmd, Shell::Command] {
        assert_eq!(synopsis().generate(shell).unwrap(), expected, "{shell}");
    }
}

#[test]
fn synopsis_powershell() {
    let expected = "\
# this is my config file
$env:FOO = \"bar\"
$env:PERL5LIB = \"/foo/bar/lib/perl5;/foo/bar/lib/perl5/perl5/site\"
if($env:PATH) { $env:PATH = $env:PATH + \";/foo/bar/bin;/bar/foo/bin\" } else { $env:PATH = \"/foo/bar/bin;/bar/foo/bin\" }
";
    assert_eq!(synopsis().generate(Shell::Power).unwrap(), expected);
}

#[test]
fn prepend_path() {
    let build = || {
        let mut config = ShellConfig::new();
        config.prepend_path("PATH", ["/a/bin", "/b/bin"]);
        config
    };

    assert_eq!(
        build().generate(Shell::Bourne).unwrap(),
        "\
if [ -n \"$PATH\" ] ; then
  PATH='/a/bin:/b/bin:'$PATH;
  export PATH;
else
  PATH='/a/bin:/b/bin';
  export PATH;
fi;
"
    );
    assert_eq!(
        build().generate(Shell::C).unwrap(),
        "test \"$?PATH\" = 0 && setenv PATH '/a/bin:/b/bin' || setenv PATH '/a/bin:/b/bin:'\"$PATH\";\n"
    );
    assert_eq!(
        build().generate(Shell::Fish).unwrap(),
        "if [ \"$PATH\" == \"\" ]; set -x PATH /a/bin /b/bin; else; set -x PATH /a/bin /b/bin $PATH;end\n"
    );
    assert_eq!(
        build().generate(Shell::Cmd).unwrap(),
        "if defined PATH (set PATH=/a/bin;/b/bin;%PATH%) else (set PATH=/a/bin;/b/bin)\n"
    );
    assert_eq!(
        build().generate(Shell::Power).unwrap(),
        "if($env:PATH) { $env:PATH = \"/a/bin;/b/bin;\" + $env:PATH } else { $env:PATH = \"/a/bin;/b/bin\" }\n"
    );
}

#[test]
fn append_single_element() {
    let build = || {
        let mut config = ShellConfig::new();
        config.append_path("X", ["/one"]);
        config
    };

    assert_eq!(
        build().generate(Shell::Bourne).unwrap(),
        "\
if [ -n \"$X\" ] ; then
  X=$X':/one';
  export X
else
  X='/one';
  export X;
fi;
"
    );
    assert_eq!(
        build().generate(Shell::C).unwrap(),
        "test \"$?X\" = 0 && setenv X '/one' || setenv X \"$X\"':/one';\n"
    );
    assert_eq!(
        build().generate(Shell::Fish).unwrap(),
        "if [ \"$X\" == \"\" ]; set -x X /one; else; set -x X $X /one;end\n"
    );
    assert_eq!(
        build().generate(Shell::Cmd).unwrap(),
        "if defined X (set X=%X%;/one) else (set X=/one)\n"
    );
    assert_eq!(
        build().generate(Shell::Power).unwrap(),
        "if($env:X) { $env:X = $env:X + \";/one\" } else { $env:X = \"/one\" }\n"
    );
}

#[test]
fn empty_path_list_is_a_noop() {
    let mut config = ShellConfig::new();
    config.append_path("PATH", Vec::<String>::new());
    config.prepend_path("PATH", Vec::<String>::new());
    assert_eq!(config.generate(Shell::Bourne).unwrap(), "");
}

#[test]
fn set_alias() {
    let build = || {
        let mut config = ShellConfig::new();
        config.set_alias("ll", "ls -l");
        config
    };

    assert_eq!(
        build().generate(Shell::Bourne).unwrap(),
        "alias ll=\"ls -l\";\n"
    );
    assert_eq!(build().generate(Shell::C).unwrap(), "alias ll ls -l;\n");
    assert_eq!(
        build().generate(Shell::Fish).unwrap(),
        "alias ll 'ls -l';\n"
    );
    assert_eq!(
        build().generate(Shell::Cmd).unwrap(),
        "DOSKEY ll=ls -l $*\n"
    );
    assert_eq!(
        build().generate(Shell::Power).unwrap(),
        "function ll { ls -l $args }\n"
    );
}

#[test]
fn alias_powershell_escapes_command() {
    let mut config = ShellConfig::new();
    config.set_alias("x", "foo \"bar\"");
    assert_eq!(
        config.generate(Shell::Power).unwrap(),
        "function x { foo `\"bar`\" $args }\n"
    );
}

fn escaping() -> ShellConfig {
    let mut config = ShellConfig::new();
    config.set("Q", "it's a \"test\"");
    config.set("NL", "line1\nline2");
    config.set("SPECIAL", "a&b^c|d<e>f(g)h%i`j$k#l!m");
    config
}

#[test]
fn escaping_bourne() {
    assert_eq!(
        escaping().generate(Shell::Bourne).unwrap(),
        "\
Q='it'\"'\"'s a \"test\"';
export Q;
NL='line1
line2';
export NL;
SPECIAL='a&b^c|d<e>f(g)h%i`j$k#l!m';
export SPECIAL;
"
    );
}

#[test]
fn escaping_c() {
    assert_eq!(
        escaping().generate(Shell::C).unwrap(),
        "\
setenv Q 'it'\"'\"'s a \"test\"';
setenv NL 'line1\\
line2';
setenv SPECIAL 'a&b^c|d<e>f(g)h%i`j$k#l\\!m';
"
    );
}

#[test]
fn escaping_fish() {
    assert_eq!(
        escaping().generate(Shell::Fish).unwrap(),
        "\
set -x Q 'it'\"'\"'s a \"test\"';
set -x NL 'line1\\
line2';
set -x SPECIAL 'a&b^c|d<e>f(g)h%i`j$k#l!m';
"
    );
}

#[test]
fn escaping_cmd() {
    assert_eq!(
        escaping().generate(Shell::Cmd).unwrap(),
        "\
set Q=it's a \"test\"
set NL=line1^

line2
set SPECIAL=a^&b^^c^|d^<e^>f^(g^)h%%i`j$k#l!m
"
    );
}

#[test]
fn escaping_powershell() {
    assert_eq!(
        escaping().generate(Shell::Power).unwrap(),
        "\
$env:Q = \"it`'s a `\"test`\"\"
$env:NL = \"line1`nline2\"
$env:SPECIAL = \"a&b^c|d<e>f`(g`)h%i``j`$k`#l!m\"
"
    );
}

#[test]
fn shebang() {
    let mut with_default = ShellConfig::new();
    with_default.shebang();
    assert_eq!(with_default.generate(Shell::Bourne).unwrap(), "#!/bin/sh\n");

    let mut with_location = ShellConfig::new();
    with_location.shebang_location("/usr/bin/bash");
    assert_eq!(
        with_location.generate(Shell::Bash).unwrap(),
        "#!/usr/bin/bash\n"
    );

    // Ignored for non-UNIX shells.
    let mut ignored = ShellConfig::new();
    ignored.shebang();
    assert_eq!(ignored.generate(Shell::Cmd).unwrap(), "");
}

#[test]
fn echo_off() {
    let build = || {
        let mut config = ShellConfig::new();
        config.echo_off();
        config.set("A", "b");
        config
    };

    assert_eq!(
        build().generate(Shell::Cmd).unwrap(),
        "@echo off\nset A=b\n"
    );
    assert_eq!(
        build().generate(Shell::Command).unwrap(),
        "@echo off\nset A=b\n"
    );
    // Not a DOS/Windows shell: no `@echo off`.
    assert_eq!(
        build().generate(Shell::Bourne).unwrap(),
        "A='b';\nexport A;\n"
    );
}

#[test]
fn set_path_sep() {
    let mut bourne = ShellConfig::new();
    bourne.set_path_sep(",");
    bourne.set_path("X", ["a", "b", "c"]);
    assert_eq!(
        bourne.generate(Shell::Bourne).unwrap(),
        "X='a,b,c';\nexport X;\n"
    );

    let mut cmd = ShellConfig::new();
    cmd.set_path_sep("#");
    cmd.append_path("X", ["a", "b"]);
    assert_eq!(
        cmd.generate(Shell::Cmd).unwrap(),
        "if defined X (set X=%X%#a#b) else (set X=a#b)\n"
    );
}

#[test]
fn comments() {
    let mut multiline = ShellConfig::new();
    multiline.comment("one\ntwo\nthree");
    assert_eq!(
        multiline.generate(Shell::Bourne).unwrap(),
        "# one\n# two\n# three\n"
    );

    let mut many = ShellConfig::new();
    many.comments(["a", "b"]);
    assert_eq!(many.generate(Shell::Cmd).unwrap(), "rem a\nrem b\n");

    let mut power = ShellConfig::new();
    power.comment("a\nb");
    assert_eq!(power.generate(Shell::Power).unwrap(), "# a\n# b\n");
}

#[test]
fn trailing_newline_in_comment_does_not_add_blank_line() {
    let mut config = ShellConfig::new();
    config.comment("only\n");
    assert_eq!(config.generate(Shell::Bourne).unwrap(), "# only\n");
}

#[test]
fn set_path_single_value() {
    let mut config = ShellConfig::new();
    config.set_path("X", ["/one"]);
    assert_eq!(
        config.generate(Shell::Bourne).unwrap(),
        "X='/one';\nexport X;\n"
    );
}

#[test]
fn dcl_rejects_every_operation() {
    let mut set = ShellConfig::new();
    set.set("A", "b");
    let err = set.generate(Shell::Dcl).unwrap_err();
    assert!(matches!(
        err,
        GenerateError::Unsupported {
            operation: "set",
            shell: Shell::Dcl
        }
    ));
    assert_eq!(err.to_string(), "don't know how to \"set\" with dcl");

    let mut comment = ShellConfig::new();
    comment.comment("x");
    assert_eq!(
        comment.generate(Shell::Dcl).unwrap_err().to_string(),
        "don't know how to \"comment\" with dcl"
    );

    let mut alias = ShellConfig::new();
    alias.set_alias("a", "b");
    assert_eq!(
        alias.generate(Shell::Dcl).unwrap_err().to_string(),
        "don't know how to \"alias\" with dcl"
    );

    let mut path = ShellConfig::new();
    path.append_path("P", ["x"]);
    assert_eq!(
        path.generate(Shell::Dcl).unwrap_err().to_string(),
        "don't know how to \"append_path\" with dcl"
    );
}

#[test]
fn generate_file_round_trips() {
    let dir = std::env::temp_dir().join(format!("scg-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.sh");

    let mut config = ShellConfig::new();
    config.set("FOO", "bar");
    config.generate_file(Shell::Bourne, &path).unwrap();

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "FOO='bar';\nexport FOO;\n"
    );
    std::fs::remove_dir_all(&dir).ok();
}
