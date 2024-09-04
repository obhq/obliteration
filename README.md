<p align="center"><img alt="Logo" src="logo.png"></p>

## About
[![CI](https://github.com/obhq/obliteration/actions/workflows/main.yml/badge.svg)](https://github.com/obhq/obliteration/actions/workflows/main.yml)
[![Zulip](https://img.shields.io/badge/zulip-join_chat-brightgreen.svg)](https://obkrnl.zulipchat.com)

Obliteration is a free and open-source PlayStation 4 kernel rewritten in Rust. Its goal is to run the PlayStation 4 system software on Windows, Linux and macOS using a custom made virtualization stack optimized specifically for Obliteration. **The project is under development and cannot run any games yet**. The reason it take so long is because we decided to go with the correct path without stubbing as much as possible.

This project started as a hard-fork from [Kyty](https://github.com/InoriRus/Kyty). Then we decided to rewrite the whole project from scratch by using Kyty and [Uplift](https://github.com/idc/uplift) as a reference to help us getting started with the project.

The project logo and icon was designed by [VocalFan](https://github.com/VocalFan).

## Get a daily build

You can download binaries from the latest commits [here](https://github.com/obhq/obliteration/actions/workflows/main.yml). You **MUST** sign in to GitHub otherwise you will not be able to download files.

## System requirements

- Windows 10, Linux or macOS 11+.
  - On Windows and Linux make sure you have Vulkan 1.3 installed. If you encountered `Failed to initialize Vulkan (-9)` that mean you don't have a Vulkan installed.
- x86-64 CPU. We want to support non-x86 but currently we don't have any developers who are willing to work on this.
- CPU with hardware virtualization supports.
  - Windows and Linux users may need to enable this feature on the BIOS/UEFI settings.
- A PS4 with system software version 11.00 for firmware dumping.

### Windows-specific requirements

- [Microsoft Visual C++ 2022 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). If there is an error related to `msvcp140.dll`, `vcruntime140.dll`, or `vcruntime140_1.dll` that means you need to install this manually. It's likely your system already has it, so try to run Obliteration first.
- [Virtual Machine Platform](https://github.com/obhq/obliteration/wiki/Common-Issues)

## Building and Development

Information on building Obliteration and preparing to be a developer can be found on our [Wiki.](https://github.com/obhq/obliteration/wiki/Compilation-&-Development)

## License

- `src/ansi_escape.hpp`, `src/ansi_escape.cpp`, `src/log_formatter.hpp` and `src/log_formatter.cpp` are licensed under GPL-3.0 only.
- `src/param`, `src/pfs` and `src/pkg` are licensed under LGPL-3.0.
- All other source code are licensed under either MIT License or Apache License, Version 2.0; or both. If the file header does not specify which license then it is licensed under MIT License.
- All release binaries are under GPL-3.0.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Obliteration by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

### UI Icons

We use icons from https://materialdesignicons.com for UI (e.g. on the menu and toolbar).
