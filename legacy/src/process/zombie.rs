/// States of a process when it is a zombie.
#[derive(Debug)]
pub struct ZombieProc {}

impl ZombieProc {
    pub(super) fn new() -> Self {
        Self {}
    }
}
