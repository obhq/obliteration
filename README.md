<p align="center">
  <img alt="Obliteration" src=".github/ReadME/obliteration-full-icon.png">
</p>

---

## About

Obliteration is an experimental PS4 emulator using [Kyty](https://github.com/InoriRus/Kyty) and [Uplift](https://github.com/idc/uplift) as a reference. The project is under development and cannot run any games that Kyty is able to run yet.

**The original author of Kyty does not involved in the development of Obliteration in anyway.** Obliteration is a completely separated project. The reason you see the author of Kyty in a contributor list because this project contains commits from Kyty.

## Get a daily build

You can download Windows binary for the latest development [here](https://github.com/ultimaweapon/obliteration/actions/workflows/main.yml).

[![CI](https://github.com/ultimaweapon/obliteration/actions/workflows/main.yml/badge.svg)](https://github.com/ultimaweapon/obliteration/actions/workflows/main.yml)

## Screenshots

![Game list](screenshots/game-list.png)

Thanks [Mou-Ikkai](https://github.com/Mou-Ikkai) for the awesome icon!

## Features

- [x] Built-in PKG file supports for Fake PKG.
- [x] Game library.
- [x] Direct mounting PFS image.
- [ ] Emulate system calls instead of user space libraries.

## System requirements

- Windows 10 x64 or Linux x86-64.
- CPU that supports all of instructions on [PS4 CPU](https://en.wikipedia.org/wiki/Jaguar_(microarchitecture)#Instruction_set_support).
  - AMD:
    - Minimum (Based on Required Instruction Sets): Jaguar-Based CPUs or newer
    - Recommended (Based on Performance): Zen-Based CPUs or newer
  - Intel:
    - Minimum (Based on Required Instruction Sets): Haswell-Based CPUs or newer
    - Recommended (Based on Performance): 5th Gen CPUs or newer

### Windows specific requirements

- [Microsoft Visual C++ 2019 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). It is likely your system already have it so try to run Obliteration first. If there is an error related to `msvcp140.dll`, `vcruntime140.dll` or `vcruntime140_1.dll` that mean you need to install this manually.

## Building from source

### Windows prerequisites

- Visual Studio 2019
- Rust 1.63
- CMake 3.16

### Linux prerequisites

- GCC 9.4
- Rust 1.63
- CMake 3.16

### Install Qt 6.2

You need to install Qt 6.2 manually on your system before you proceed.

### Install Vulkan SDK

For Windows just download from https://vulkan.lunarg.com/sdk/home. Once installed you need to restart your computer to make `VULKAN_SDK` environment variable effective.

For Linux it will be depend on your distro. For Arch Linux just install `vulkan-devel` and set `VULKAN_SDK` to `/usr`. For other distro try to find in its package repository first. If not found visit https://vulkan.lunarg.com/sdk/home to download and install it manually.

### Clone the repository

You need to clone this repository with submodules like this:

```sh
git clone --recurse-submodules https://github.com/ultimaweapon/obliteration.git
```

### Initialize VCPKG

Windows:

```pwsh
.\vcpkg\bootstrap-vcpkg.bat
```

Linux:

```sh
./vcpkg/bootstrap-vcpkg.sh
```

### Install external dependencies

Windows:

```pwsh
.\vcpkg-restore.ps1
```

Linux:

```sh
./vcpkg-restore.sh
```

If the above command produced an error about Vulkan SDK that mean you have improper Vulkan SDK installed.

### Configure build system

Windows:

```pwsh
cmake -B build -A x64
```

Linux:

```sh
cmake -B build -D CMAKE_BUILD_TYPE=Release
```

### Build

Windows:

```pwsh
cmake --build build --config Release
```

Linux:

```sh
cmake --build build
```

## Development

We recommended Visual Studio Code as a code editor with the following extensions:

- [C/C++](https://marketplace.visualstudio.com/items?itemName=ms-vscode.cpptools)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [CMake](https://marketplace.visualstudio.com/items?itemName=twxs.cmake)
- [CMake Tools](https://marketplace.visualstudio.com/items?itemName=ms-vscode.cmake-tools)
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)

Remove `build` directory from the previous step and open this folder with VS Code. Everything should work out of the box (e.g. code completion, debugging, etc).

### Get a homebrew application for testing

If you don't have a PS4 application for testing you can download PS Scene Quiz for free [here](https://pkg-zone.com/details/LAPY10010).

### Rules for Rust sources

- Don't be afraid to use `unsafe` when it is necessary. We are written an application that required very high performance code and we use Rust to assist us on this task, not to use Rust to compromise performance that C/C++ can provides.
- Any functions that operate on pointers don't need to mark as `unsafe`. The reasons is because it will required the caller to wrap it in `unsafe`. We already embrace `unsafe` code so no point to make it harder to use.
- Don't chain method calls without intermidate variable if the result code is hard to follow. We encourage code readability than a pleasure when writing so try to make it easy to read and understand for other people.
- Do not blindly cast an integer. Make sure the value can be fit in a destination type. We don't have any plans to support non 64-bits system so the pointer size and it related types like `usize` is always 64-bits.

### Rules for C++ sources

Just follow how Qt is written (e.g. coding style, etc.). Always prefers Qt classes over `std` when possible so you don't need to handle exception. Do not use Qt `ui` file to design the UI because it will break on high-DPI screen.

### Starting point

The application consist of 2 binaries:

1. Main application. This is what user will see when they launch Obliteration. Its entry point is inside `src/main.cpp`.
2. Emulator kernel. This is where PS4 emulation take place. Its entry point is inside `src/kernel/src/main.rs`.

### Debugging kernel

Create `.kernel-debug` in the root of repository. The content of this file is YAML and the kernel will deserialize it to `Args` struct in `src/kernel/src/main.rs` when passing `--debug` flag to the kernel. We already provided a launch configuration for VS Code so all you need to do is just choose `Kernel` as a configuration and start debugging.

### Action icons

We use icons from https://materialdesignicons.com for action icon (e.g. on menu and toolbar).

## License

- All source code except `src/pfs` and `src/pkg` are licensed under MIT license.
- `src/pfs` and `src/pkg` licensed under LGPL-3.0 license.
- All release binaries are under GPL-3.0 license.
