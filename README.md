<p align="center"><img alt="Logo" src="gui/ui/assets/logo.png"></p>

## About
[![CI](https://github.com/obhq/obliteration/actions/workflows/main.yml/badge.svg)](https://github.com/obhq/obliteration/actions/workflows/main.yml)
[![Zulip](https://img.shields.io/badge/zulip-join_chat-brightgreen.svg)](https://obkrnl.zulipchat.com)

Obliteration is a free and open-source PlayStation 4 kernel rewritten in Rust. Our goal is not running on the PlayStation 4 but to run the dumped PlayStation 4 system software on Windows, Linux and macOS using a custom made virtualization stack optimized specifically for Obliteration.

<p align="center"><img alt="Architecture" src="architecture.svg"></p>

This project started as a hard-fork from [Kyty](https://github.com/InoriRus/Kyty). Then we decided to rewrite the whole project from scratch by using Kyty and [Uplift](https://github.com/idc/uplift) as a reference to help us getting started with the project.

Our ultimate goal is to become a permissive free and open-source operating system optimized for gaming that can run on a variety of hardware. The reason we want to built this because:

- Windows is bloated and Microsoft keep pushing too many things into it.
- Linux is a nightmare for beginners. Its license also making it not an ideal choice for a proprietary hardware.
- macOS has a limited set of hardware and its price too expensive. You can get a PC with high-end graphic card at the same price.
- FreeBSD and the others was not designed for gaming. Their goal are either a server or a general desktop.

So we want to take this opportunity to go beyond a PlayStation 4 emulator since we already building an operating system kernel.

The project logo and icon was designed by [VocalFan](https://github.com/VocalFan).

## Status

Currently we cannot run any games yet. What we have is a working [64-bit](https://en.wikipedia.org/wiki/Long_mode) kernel and the VMM to run it. The kernel has been successfully setup [GDT](https://en.wikipedia.org/wiki/Global_Descriptor_Table), [TSS](https://en.wikipedia.org/wiki/Task_state_segment), [IDT](https://en.wikipedia.org/wiki/Interrupt_descriptor_table) and [syscall](https://en.wikipedia.org/wiki/System_call) instruction. Right now we are working on [UMA](https://man.freebsd.org/cgi/man.cgi?query=uma) system. Once this finished we will start migrating code from our [legacy user-mode kernel](https://github.com/obhq/obliteration/tree/main/legacy/src) then execute `mini-syscore.elf`.

The reason it take so long is because we try to implement the kernel without stubbing as much as possible.

## Key features

- Cross-platform with native binary for each platform.
- On-demand memory allocation instead of pre-allocated 8 GB at startup.
- Near-native performance by using [Windows Hypervisor Platform](https://learn.microsoft.com/en-us/virtualization/api/#windows-hypervisor-platform), [KVM](https://en.wikipedia.org/wiki/Kernel-based_Virtual_Machine) or [Hypervisor Framework](https://developer.apple.com/documentation/hypervisor) directly with custom made virtual devices for optimized MMIO.
- Kernel behavior is near-identical to the PlayStation 4 kernel. Although we can't run any game yet but we believe the choice we made here will allows us to have very high compatibility.

## Get a daily build

Please note that we cannot run any games yet as stated on the above. But if you want to try or help on testing you can download binaries from the latest commits [here](https://github.com/obhq/obliteration/actions/workflows/main.yml). You **MUST** sign in to GitHub otherwise you will not be able to download files.

Our developers are using Linux so Windows and macOS users may encountered some unimplemented functions. A PR to implement those functions is welcome or you can report an issue if you would like to be a tester so we can try implement it for you to test.

## Building and Development

Information related to Obliteration development and building from source can be found on our [developer documentation](https://dev.obliteration.net).

### UI Icons

We use icons from https://materialdesignicons.com for UI.

## License

All source code are licensed under either MIT License or Apache License, Version 2.0; or both.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Obliteration by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

Note that we can't accept any code from the other PlayStation 4 emulators if they are licensed under other license than MIT or Apache-2.0 unless you are the author of that code.
