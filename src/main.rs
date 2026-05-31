use jscpd_rs::{app, report};

fn main() {
    match app::run_current_process() {
        Ok(outcome) => {
            if let Some(code) = outcome.exit_code
                && code != 0
            {
                std::process::exit(code);
            }
        }
        Err(error) => {
            if let Some(threshold) = error.downcast_ref::<report::ThresholdExceeded>() {
                eprintln!("{}", threshold.message());
                std::process::exit(1);
            }
            let message = error.to_string();
            if let Some(stdout_error) = app::upstream_stdout_error(&message) {
                println!("{stdout_error}");
                std::process::exit(1);
            }
            eprintln!("error: {error:#}");
            std::process::exit(1);
        }
    }
}
