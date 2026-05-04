use std::num::NonZero;

/// Provides methods to handle debug events.
pub trait GdbHandler {
    fn active_threads(&mut self) -> impl IntoIterator<Item = NonZero<usize>>;
    async fn suspend_threads(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    #[cfg(target_arch = "x86_64")]
    async fn read_rax(&mut self, td: NonZero<usize>) -> Result<usize, Box<dyn std::error::Error>>;
    #[cfg(target_arch = "x86_64")]
    async fn read_rip(&mut self, td: NonZero<usize>) -> Result<usize, Box<dyn std::error::Error>>;
    #[cfg(target_arch = "x86_64")]
    async fn read_rsp(&mut self, td: NonZero<usize>) -> Result<usize, Box<dyn std::error::Error>>;
    async fn read_memory(
        &mut self,
        td: NonZero<usize>,
        addr: usize,
        len: NonZero<usize>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}
