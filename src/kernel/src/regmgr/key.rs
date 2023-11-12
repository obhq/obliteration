use std::fmt::{Display, Formatter};

/// A unique identifier for a registry entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegKey(u32);

#[allow(dead_code)] // Removes complaints about unused Regkeys.
impl RegKey {
    // pub const REGISTRY_VERSION: Self = Self(0);
    pub const REGISTRY_VERSION: Self = Self(0x01010000);
    pub const REGISTRY_INSTALL: Self = Self(0x01020000);
    pub const REGISTRY_UPDATE: Self = Self(0x01030000);
    pub const REGISTRY_NOT_SAVE: Self = Self(0x01040000);
    pub const REGISTRY_RECOVER: Self = Self(0x01050000);
    pub const REGISTRY_DOWNGRADE: Self = Self(0x01060000);
    pub const REGISTRY_BOOTCOUNT: Self = Self(0x01070000);
    pub const REGISTRY_LASTVER: Self = Self(0x01080000);
    pub const REGISTRY_INIT_FLAG: Self = Self(0x01400000);
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
    pub const NP_DEBUG: Self = Self(0x19810000);
    pub const BROWSER_DEBUG_NOTIFICATION: Self = Self(0x3CC80700);
    pub const DEVENV_TOOL_BOOT_PARAM: Self = Self(0x78020300);
    pub const DEVENV_TOOL_TRC_NOTIFY: Self = Self(0x78026400);
    pub const DEVENV_TOOL_USE_DEFAULT_LIB: Self = Self(0x78028300);
    pub const DEVENV_TOOL_SYS_PRX_PRELOAD: Self = Self(0x78028A00);
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
            Self::REGISTRY_VERSION => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_version"),
            Self::REGISTRY_INSTALL => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_install"),
            Self::REGISTRY_UPDATE => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_update"),
            Self::REGISTRY_NOT_SAVE => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_not_save"),
            Self::REGISTRY_RECOVER => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_recover"),
            Self::REGISTRY_DOWNGRADE => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_downgrade"),
            Self::REGISTRY_BOOTCOUNT => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_bootcount"),
            Self::REGISTRY_LASTVER => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_lastver"),
            Self::REGISTRY_INIT_FLAG => f.write_str("SCE_REGMGR_ENT_KEY_REGISTRY_init_flag"),
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
            Self::NP_DEBUG => f.write_str("SCE_REGMGR_ENT_KEY_NP_debug"),
            Self::BROWSER_DEBUG_NOTIFICATION => {
                f.write_str("SCE_REGMGR_ENT_KEY_BROWSER_DEBUG_notification")
            }
            Self::DEVENV_TOOL_BOOT_PARAM => {
                f.write_str("SCE_REGMGR_ENT_KEY_DEVENV_TOOL_boot_param")
            }
            Self::DEVENV_TOOL_TRC_NOTIFY => {
                f.write_str("SCE_REGMGR_ENT_KEY_DEVENV_TOOL_trc_notify")
            }
            Self::DEVENV_TOOL_USE_DEFAULT_LIB => {
                f.write_str("SCE_REGMGR_ENT_KEY_DEVENV_TOOL_use_default_lib")
            }
            Self::DEVENV_TOOL_SYS_PRX_PRELOAD => {
                f.write_str("SCE_REGMGR_ENT_KEY_DEVENV_TOOL_sys_prx_preload")
            }
            Self::DEVENV_TOOL_GAME_INTMEM_DBG => {
                f.write_str("SCE_REGMGR_ENT_KEY_DEVENV_TOOL_game_intmem_dbg")
            }
            v => write!(f, "{:#x}", v.0),
        }
    }
}
