use crate::config::Config;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use config::KernelMap;
use core::arch::asm;

pub fn identify_cpu() -> CpuInfo {
    // Load MIDR_EL1.
    let id: u64;

    unsafe { asm!("mrs {v}, MIDR_EL1", v = out(reg) id, options(pure, nomem, nostack)) };

    // Get vendor.
    let cpu_vendor = match (id & 0xFF000000) >> 24 {
        0x41 => "Arm Limited".into(),
        0x42 => "Broadcom Corporation".into(),
        0x43 => "Cavium Inc".into(),
        0x44 => "Digital Equipment Corporation".into(),
        0x46 => "Fujitsu Ltd".into(),
        0x49 => "Infineon Technologies AG".into(),
        0x4D => "Motorola/Freescale Semiconductor Inc".into(),
        0x4E => "NVIDIA Corporation".into(),
        0x50 => "Applied Micro Circuits Corporation".into(),
        0x51 => "Qualcomm Inc".into(),
        0x56 => "Marvell International Ltd".into(),
        0x61 => "Apple Inc".into(),
        0x69 => "Intel Corporation".into(),
        0xC0 => "Ampere Computing".into(),
        v => format!("Unknown {v:#x}"),
    };

    CpuInfo {
        cpu_vendor,
        cpu_id: (id & 0xFFFFFFFF) as u32,
    }
}

pub unsafe fn setup_main_cpu(
    config: &Config,
    cpu: CpuInfo,
    map: &'static KernelMap,
) -> Arc<ArchConfig> {
    todo!()
}

/// Contains information for CPU on current machine.
pub struct CpuInfo {
    pub cpu_vendor: String,
    pub cpu_id: u32,
}

/// Contains architecture-specific configurations obtained from [`setup_main_cpu()`].
pub struct ArchConfig {
    pub secondary_start: &'static [u8],
}
