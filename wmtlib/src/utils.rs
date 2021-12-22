pub fn log_if_verbose(verbose: bool, message: &str) {
    match verbose {
        true => tracing::info!(message),
        false => {}
    }
}
