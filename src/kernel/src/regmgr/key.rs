use std::fmt::{Display, Formatter};

/// A unique identifier for a registry entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegKey(u32);

impl RegKey {
    pub const REGISTRY_RECOVER: Self = Self(0x01050000);
    pub const SYSTEM_UPDATE_MODE: Self = Self(0x02010000);
    pub const SYSTEM_POWER_SHUTDOWN_STATUS: Self = Self(0x02820E00);
    pub const SYSTEM_SPECIFIC_IDU_MODE: Self = Self(0x02860100);
    pub const SYSTEM_SPECIFIC_SHOW_MODE: Self = Self(0x02860200);
    pub const SYSTEM_SPECIFIC_ARCADE_MODE: Self = Self(0x02860300);
    pub const SYSTEM_LIBC_INTMEM_PEAK_SIZE: Self = Self(0x02C30100);
    pub const SYSTEM_LIBC_INTMEM_SHORTAGE_COUNT: Self = Self(0x02C30200);
    pub const AUDIOOUT_CONNECTOR_TYPE: Self = Self(0x0B060000);
    pub const AUDIOOUT_CODEC: Self = Self(0x0B070000);
    pub const NET_WIFI_FREQ_BAND: Self = Self(0x141E0500);
    pub const DEVENV_TOOL_BOOT_PARAM: Self = Self(0x78020300);
    pub const DEVENV_TOOL_GAME_INTMEM_DBG: Self = Self(0x7802BF00);

    pub(super) const fn new(v: u32) -> Self {
        Self(v)
    }

    pub fn value(self) -> u32 {
        self.0
    }
}

impl Display for RegKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::REGISTRY_RECOVER => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_recover"),
            Self::SYSTEM_UPDATE_MODE => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_update_mode"),
            Self::SYSTEM_POWER_SHUTDOWN_STATUS => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_POWER_shutdown_status")
            }
            Self::SYSTEM_SPECIFIC_IDU_MODE => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_SPECIFIC_idu_mode")
            }
            Self::SYSTEM_SPECIFIC_SHOW_MODE => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_SPECIFIC_show_mode")
            }
            Self::SYSTEM_SPECIFIC_ARCADE_MODE => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_SPECIFIC_arcade_mode")
            }
            Self::SYSTEM_LIBC_INTMEM_PEAK_SIZE => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_LIBC_intmem_peak_size")
            }
            Self::SYSTEM_LIBC_INTMEM_SHORTAGE_COUNT => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_LIBC_intmem_shortage_count")
            }
            Self::AUDIOOUT_CONNECTOR_TYPE => {
                f.write_str("SCE_REGMGR_ENT_KEY_AUDIOOUT_connector_type")
            }
            Self::AUDIOOUT_CODEC => f.write_str("SCE_REGMGR_ENT_KEY_AUDIOOUT_codec"),
            Self::NET_WIFI_FREQ_BAND => f.write_str("SCE_REGMGR_ENT_KEY_NET_WIFI_freq_band"),
            Self::DEVENV_TOOL_BOOT_PARAM => {
                f.write_str("SCE_REGMGR_ENT_KEY_DEVENV_TOOL_boot_param")
            }
            Self::DEVENV_TOOL_GAME_INTMEM_DBG => {
                f.write_str("SCE_REGMGR_ENT_KEY_DEVENV_TOOL_game_intmem_dbg")
            }
            v => write!(f, "{:#x}", v.0),
        }
    }
}
