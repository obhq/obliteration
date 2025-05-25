use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Model of the CPU to report to the kernel.
///
/// This has no effect on non-x86 and the kernel always assume [`CpuModel::Pro`].
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuModel {
    Host,
    Pro,
    ProWithHost,
}

impl Display for CpuModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            Self::Host => "Host",
            Self::Pro => "PlayStation 4 Pro",
            Self::ProWithHost => "PlayStation 4 Pro (Host Features)",
        };

        f.write_str(v)
    }
}
