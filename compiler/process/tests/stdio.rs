use std::io::{Read, Write};
use std::process::Command;

use scoop_process::CommandExt;

const PIPE_BYTES: usize = 2 * 1024 * 1024;

#[test]
fn large_input_and_both_output_pipes_make_progress() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "stdio_child", "--nocapture"])
        .env("SCOOP_STDIO_TEST", "echo")
        .scoop_output_with_input(&vec![b'i'; PIPE_BYTES])
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.ends_with(b"input received\n"));
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == 0xa5).count(),
        PIPE_BYTES
    );
    assert_eq!(output.stderr, vec![b'e'; PIPE_BYTES]);
}

#[test]
fn early_failure_preserves_the_tools_diagnostic() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "stdio_child", "--nocapture"])
        .env("SCOOP_STDIO_TEST", "fail")
        .scoop_output_with_input(&vec![b'i'; PIPE_BYTES])
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert!(output.stderr.ends_with(b"invalid source\n"));
}

#[test]
fn stdio_child() {
    let Some(mode) = std::env::var_os("SCOOP_STDIO_TEST") else {
        return;
    };
    if mode == "fail" {
        std::io::stderr().write_all(b"invalid source\n").unwrap();
        std::process::exit(7);
    }
    std::io::stdout()
        .write_all(&vec![0xa5; PIPE_BYTES])
        .unwrap();
    std::io::stderr()
        .write_all(&vec![b'e'; PIPE_BYTES])
        .unwrap();
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    assert_eq!(input, vec![b'i'; PIPE_BYTES]);
    std::io::stdout().write_all(b"input received\n").unwrap();
    std::process::exit(0);
}
