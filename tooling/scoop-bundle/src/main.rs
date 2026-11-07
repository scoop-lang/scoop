use clap::Parser;

fn main() -> std::process::ExitCode {
    let args = scoop_bundle::Args::parse();
    match scoop_bundle::run(&args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if args.json() {
                println!(
                    "{}",
                    serde_json::json!({"schema": 1, "kind": "bundle-error", "message": format!("{error:#}")})
                );
            }
            eprintln!("scoop-bundle: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
