/// An implementation of `appinfo` structure on the PS4.
#[derive(Debug)]
pub struct AppInfo {
    unk1: u32, // 0x02 = ET_SCE_REPLAY_EXEC
    title_id: String,
}

impl AppInfo {
    pub fn new() -> Self {
        Self {
            unk1: 0,
            title_id: String::new(),
        }
    }

    pub fn unk1(&self) -> u32 {
        self.unk1
    }

    pub fn serialize(&self) -> [u8; 72] {
        // TODO: Right now we don't know how appinfo is structured but it seems like it is safe to
        // fill it with zeroes.
        [0; 72]
    }
}
