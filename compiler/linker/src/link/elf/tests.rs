use super::*;
use std::process::Command;

#[test]
fn whole_archive_preserves_initializers_and_quoted_paths() {
    let directory = tempfile::tempdir().unwrap();
    let quoted = directory
        .path()
        .join("objects, spaces 'single' \"double\" \\");
    std::fs::create_dir(&quoted).unwrap();
    let source = quoted.join("main.c");
    let main = quoted.join("main.o");
    let initializer = quoted.join("initializer.o");
    std::fs::write(
        &source,
        "int initialized; int main(void) { return initialized != 42; }\n",
    )
    .unwrap();
    checked(
        Command::new("gcc")
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(&main),
    );
    std::fs::write(&source, "extern int initialized; __attribute__((constructor)) static void initialize(void) { initialized = 42; }\n").unwrap();
    checked(
        Command::new("gcc")
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(&initializer),
    );
    let archive = quoted.join("selected objects.a");
    checked(
        Command::new("ar")
            .arg("rcs")
            .arg(&archive)
            .arg(main)
            .arg(initializer),
    );
    for driver in ["gcc", "g++", "musl-gcc"] {
        let scratch = quoted.join(driver);
        std::fs::create_dir(&scratch).unwrap();
        let binary = scratch.join("program");
        let mut command = Command::new(driver);
        command.arg("-Wl,-t").arg("-o").arg(&binary);
        append_archives(&mut command, &scratch, std::slice::from_ref(&archive)).unwrap();
        let output = checked(&mut command);
        let trace = std::str::from_utf8(&output.stdout).unwrap();
        assert_eq!(
            trace
                .lines()
                .filter(|line| *line == archive.to_str().unwrap())
                .count(),
            1
        );
        checked(&mut Command::new(binary));
    }
}

fn checked(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
