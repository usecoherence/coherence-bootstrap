#![allow(clippy::unwrap_used, clippy::expect_used)]

use coherence_core_db::commands::cli_parse::parse_args;

#[test]
fn parse_args_basic_flags_and_positionals() {
    let args: Vec<String> = vec![
        "--slug".into(),
        "my-slug".into(),
        "--title".into(),
        "My Title".into(),
        "positional1".into(),
        "positional2".into(),
    ];
    let parsed = parse_args(&args).expect("parse should succeed");
    assert_eq!(parsed.single_flag("slug").unwrap().unwrap(), "my-slug");
    assert_eq!(parsed.single_flag("title").unwrap().unwrap(), "My Title");
    assert_eq!(parsed.positionals, &["positional1", "positional2"]);
}

#[test]
fn parse_args_rejects_empty_flag_name() {
    let args: Vec<String> = vec!["--".into(), "value".into()];
    let result = parse_args(&args);
    assert!(result.is_err(), "empty flag name should error");
    let err = result.unwrap_err();
    assert!(
        err.contains("empty flag name"),
        "error should mention 'empty flag name'; got: {err}",
    );
}

#[test]
fn parse_args_rejects_missing_flag_value() {
    let args: Vec<String> = vec!["--slug".into()];
    let result = parse_args(&args);
    assert!(result.is_err(), "missing flag value should error");
    let err = result.unwrap_err();
    assert!(
        err.contains("missing value for --slug"),
        "error should mention missing value; got: {err}",
    );
}

#[test]
fn parse_args_rejects_value_starting_with_dash() {
    let args: Vec<String> = vec!["--slug".into(), "-not-a-value".into()];
    let result = parse_args(&args);
    assert!(result.is_err(), "value starting with dash should error");
    let err = result.unwrap_err();
    assert!(
        err.contains("missing value for --slug"),
        "error should mention missing value; got: {err}",
    );
}

#[test]
fn parse_args_multi_flag_collects_in_order() {
    let args: Vec<String> = vec![
        "--concern".into(),
        "correctness".into(),
        "--concern".into(),
        "security".into(),
        "--concern".into(),
        "performance".into(),
    ];
    let parsed = parse_args(&args).expect("parse should succeed");
    let concerns = parsed.multi_flag("concern");
    assert_eq!(concerns, &["correctness", "security", "performance"]);
}

#[test]
fn single_flag_returns_none_when_absent() {
    let args: Vec<String> = vec!["--slug".into(), "my-slug".into()];
    let parsed = parse_args(&args).expect("parse should succeed");
    assert!(parsed.single_flag("title").unwrap().is_none());
}

#[test]
fn single_flag_error_on_multiple_occurrences() {
    let args: Vec<String> = vec![
        "--slug".into(),
        "first".into(),
        "--slug".into(),
        "second".into(),
    ];
    let parsed = parse_args(&args).expect("parse should succeed");
    let result = parsed.single_flag("slug");
    assert!(result.is_err(), "single_flag on multi-flag should error");
    let err = result.unwrap_err();
    assert!(
        err.contains("--slug") && err.contains("at most once"),
        "error should mention flag name and 'at most once'; got: {err}",
    );
}

#[test]
fn multi_flag_returns_empty_when_absent() {
    let args: Vec<String> = vec!["--slug".into(), "my-slug".into()];
    let parsed = parse_args(&args).expect("parse should succeed");
    let absent = parsed.multi_flag("nonexistent");
    assert!(absent.is_empty());
}

#[test]
fn parse_args_mixed_flags_and_positionals() {
    let args: Vec<String> = vec!["--level".into(), "product".into(), "positional-only".into()];
    let parsed = parse_args(&args).expect("parse should succeed");
    assert_eq!(parsed.single_flag("level").unwrap().unwrap(), "product");
    assert_eq!(parsed.positionals, &["positional-only"]);
}
