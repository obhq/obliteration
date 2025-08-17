use super::Config;
use alloc::sync::Arc;

/// Boot time overrides that are not scaled against main memory.
pub struct Param1 {
    msgbuf_size: usize, // msgbufsize
}

impl Param1 {
    /// See `init_param1` on the Orbis for a reference.
    ///
    /// # Reference offsets
    /// | Version | Offset |
    /// |---------|--------|
    /// |PS4 11.00|0x1A5340|
    pub fn new(config: &Config) -> Arc<Self> {
        let msgbuf_size = config
            .env("kern.msgbufsize")
            .map(|v| v.parse().unwrap())
            .unwrap_or(0x10000);

        Arc::new(Self { msgbuf_size })
    }

    pub fn msgbuf_size(&self) -> usize {
        self.msgbuf_size
    }
}
