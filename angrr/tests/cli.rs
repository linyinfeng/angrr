#![cfg(unix)]

mod support;

use std::{ffi::OsString, path::PathBuf};

use angrr::command::{Interactive, Options};
use clap::Parser;
use support::run_options;

/// Behavior tests parse the CLI, so these pin the delimiter and output defaults.
#[test]
fn output_arguments_parse_delimiters() {
    assert_eq!(run_options(&[]).output_delimiter, OsString::from("\n"));
    assert_eq!(
        run_options(&["--output-delimiter=|"]).output_delimiter,
        OsString::from("|")
    );
    assert_eq!(
        run_options(&["--null-output-delimiter"]).output_delimiter,
        OsString::from("\0")
    );
    assert_eq!(
        run_options(&["--output=-"]).output,
        Some(PathBuf::from("-"))
    );
}

/// These assertions cover argument parsing only, not terminal prompting or input.
#[test]
fn interactive_cli_arguments_parse_defaults_aliases_and_overrides() {
    for (args, expected) in [
        (vec![], Interactive::Once),
        (vec!["-i"], Interactive::Always),
        (vec!["--interactive"], Interactive::Always),
        (vec!["--interactive=never"], Interactive::Never),
        (vec!["--interactive=once"], Interactive::Once),
        (vec!["--interactive=always"], Interactive::Always),
        (vec!["-n"], Interactive::Never),
        (vec!["--no-prompt"], Interactive::Never),
        (vec!["--no-prompt", "--interactive=once"], Interactive::Once),
        (
            vec!["--interactive=always", "--no-prompt"],
            Interactive::Always,
        ),
    ] {
        assert_eq!(run_options(&args).interactive, expected);
    }
    assert!(Options::try_parse_from(["angrr", "run", "--interactive=sometimes"]).is_err());
}
