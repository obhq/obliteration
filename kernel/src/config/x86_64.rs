// We use 4K 4-Level Paging here. You may wonder about this because it seems like page size on the
// PS4 is 16K. The truth is the PS4 emulate the 16K page size with 4K pages. You can check this by
// yourself by looking at acpi_install_wakeup_handler() function on the PS4 kernel and compare it
// with FreeBSD version. No idea why the PS4 choose to emulate 16K page.
pub const PAGE_SHIFT: usize = 12;
