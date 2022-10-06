use std::error::Error;

/// Print `msg` followed by `: err` to stderr.
pub fn error<'error>(pid: i32, msg: &str, err: &(dyn Error + 'error)) {
    let mut inner = err.source();

    eprint!("{}: {}: {}", pid, msg, err);

    while let Some(e) = inner {
        eprint!(" -> {}", e);
        inner = e.source();
    }

    eprintln!();
}
