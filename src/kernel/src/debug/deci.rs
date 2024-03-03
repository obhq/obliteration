use crate::fs::CharacterDevice;
use std::sync::Arc;

/// Encapsulate a deci device (e.g. `deci_stdout`).
#[allow(dead_code)]
pub struct DeciDev {
    name: &'static str,
    dev: Arc<CharacterDevice>,
}

impl DeciDev {
    pub const NAMES: [&'static str; 12] = [
        "deci_stdout",
        "deci_stderr",
        "deci_tty2",
        "deci_tty3",
        "deci_tty4",
        "deci_tty5",
        "deci_tty6",
        "deci_tty7",
        "deci_ttya0",
        "deci_ttyb0",
        "deci_ttyc0",
        "deci_coredump",
    ];

    pub(super) fn new(name: &'static str, dev: Arc<CharacterDevice>) -> Self {
        Self { name, dev }
    }
}
