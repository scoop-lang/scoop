use std::io::{self, Write};
use std::process::ExitCode;

mod cli;

fn main() -> ExitCode {
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((error, machine)) => {
            if machine {
                match error.machine_diagnostic().and_then(|value| {
                    serde_json::to_string(&value).map_err(|error| error.to_string())
                }) {
                    Ok(record) => {
                        let _ = writeln!(io::stderr(), "{record}");
                    }
                    Err(encoding) => {
                        let _ = writeln!(
                            io::stderr(),
                            "error[{}]: {error}\nerror[ICE2501]: cannot encode diagnostic: {encoding}",
                            error.code
                        );
                    }
                }
            } else {
                let _ = writeln!(io::stderr(), "error[{}]: {error}", error.code);
            }
            ExitCode::FAILURE
        }
    }
}
