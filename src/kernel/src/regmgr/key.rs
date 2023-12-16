use std::fmt::{Display, Formatter};

/// A unique identifier for a registry entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegKey(u32);

#[allow(dead_code)] // Removes complaints about unused Regkeys.
impl RegKey {
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
    pub const SYSTEM_LANGUAGE: Self = Self(0x02020000);
    pub const SYSTEM_INITIALIZE: Self = Self(0x02040000);
    pub const SYSTEM_NICKNAME: Self = Self(0x02050000);
    pub const SYSTEM_DIMMER_INTERVAL: Self = Self(0x02060000);
    pub const SYSTEM_EAPFUNCTION: Self = Self(0x02070000);
    pub const SYSTEM_ENABLE_VOICERCG: Self = Self(0x02080000);
    pub const SYSTEM_SOFT_VERSION: Self = Self(0x02090000);
    pub const SYSTEM_PROFILECH_VER: Self = Self(0x020A0000);
    pub const SYSTEM_BUTTON_ASSIGN: Self = Self(0x020B0000);
    pub const SYSTEM_BACKUP_MODE: Self = Self(0x020C0000);
    pub const SYSTEM_PON_MEMORY_TEST: Self = Self(0x020D0000);
    pub const SYSTEM_GAME_REC_MODE: Self = Self(0x020E0000);
    pub const SYSTEM_SHELL_FUNCTION: Self = Self(0x020F0000);
    pub const SYSTEM_PAD_CONNECTION: Self = Self(0x02100000);
    pub const SYSTEM_DATA_TRANSFER: Self = Self(0x02110000);
    pub const SYSTEM_BASE_MODE_CLKUP: Self = Self(0x02120000);
    pub const SYSTEM_NEO_VDDNB_VID_OFFSET: Self = Self(0x02400000);
    pub const SYSTEM_TESTBUTTON_MODE: Self = Self(0x02410000);
    pub const SYSTEM_TESTBUTTON_PARAM: Self = Self(0x02420000);
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
    pub const MORPHEUS_DEBUG_VR_CAPTURE: Self = Self(0x58800C00);
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
            Self::SYSTEM_LANGUAGE => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_language"),
            Self::SYSTEM_INITIALIZE => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_initialize"),
            Self::SYSTEM_NICKNAME => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_nickname"),
            Self::SYSTEM_DIMMER_INTERVAL => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_dimmer_interval")
            }
            Self::SYSTEM_EAPFUNCTION => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_eapfunction"),
            Self::SYSTEM_ENABLE_VOICERCG => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_enable_voicercg")
            }
            Self::SYSTEM_SOFT_VERSION => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_soft_version"),
            Self::SYSTEM_PROFILECH_VER => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_profilech_ver"),
            Self::SYSTEM_BUTTON_ASSIGN => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_button_assign"),
            Self::SYSTEM_BACKUP_MODE => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_backup_mode"),
            Self::SYSTEM_PON_MEMORY_TEST => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_pon_memory_test")
            }
            Self::SYSTEM_GAME_REC_MODE => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_game_rec_mode"),
            Self::SYSTEM_SHELL_FUNCTION => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_shell_function"),
            Self::SYSTEM_PAD_CONNECTION => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_pad_connection"),
            Self::SYSTEM_DATA_TRANSFER => f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_data_transfer"),
            Self::SYSTEM_BASE_MODE_CLKUP => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_base_mode_clkup")
            }
            Self::SYSTEM_NEO_VDDNB_VID_OFFSET => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_neo_vddnb_vid_offset")
            }
            Self::SYSTEM_TESTBUTTON_MODE => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_testbutton_mode")
            }
            Self::SYSTEM_TESTBUTTON_PARAM => {
                f.write_str("SCE_REGMGR_ENT_KEY_SYSTEM_testbutton_param")
            }
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
            Self::MORPHEUS_DEBUG_VR_CAPTURE => {
                f.write_str("SCE_REGMGR_ENT_KEY_MORPHEUS_DEBUG_vr_capture")
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
