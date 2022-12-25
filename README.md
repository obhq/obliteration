<p align="center">
  <img alt="Logo" src="logo.png">
</p>

## About
[![CI](https://github.com/ultimaweapon/obliteration/actions/workflows/main.yml/badge.svg)](https://github.com/ultimaweapon/obliteration/actions/workflows/main.yml)

Obliteration is an experimental PS4 emulator using [Kyty](https://github.com/InoriRus/Kyty) and [Uplift](https://github.com/idc/uplift) as a reference. The project is under development and cannot run any games that Kyty is able to run yet.

**The original author of Kyty is not involved in the development of Obliteration in anyway.** Obliteration is a completely separated project. The reason you see the author of Kyty in the contributor list is because this project contains commits from Kyty.

## Get a daily build

You can download the Windows binaries from the latest commits [here](https://github.com/ultimaweapon/obliteration/actions/workflows/main.yml).

## Screenshots

![Game list](screenshots/game-list.png)

Thanks [Mou-Ikkai](https://github.com/Mou-Ikkai) for the awesome icon!

## Features

- [ ] Built-in PUP file supports for decrypted PUP from [pup_decrypt](https://github.com/idc/ps4-pup_decrypt).
- [x] Built-in PKG file supports for Fake PKG.
- [x] Game library.
- [x] Direct mounting PFS image.
- [ ] Emulate system calls instead of user space libraries.

## System requirements

- Windows 10 x64, Linux x86-64, and macOS x86-64.
- CPU that supports all of the instructions on the [PS4 CPU](https://en.wikipedia.org/wiki/Jaguar_(microarchitecture)#Instruction_set_support).
  - AMD:
    - Minimum (Based on Required Instruction Sets): Jaguar-Based CPUs or newer
    - Recommended (Based on Performance): Zen-Based CPUs or newer
  - Intel:
    - Minimum (Based on Required Instruction Sets): Haswell-Based CPUs or newer
    - Recommended (Based on Performance): 5th Gen CPUs or newer

### Windows specific requirements

- [Microsoft Visual C++ 2019 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). It's likely your system already has it, so try to run Obliteration first. If there is an error related to `msvcp140.dll`, `vcruntime140.dll` or `vcruntime140_1.dll` that means you need to install this manually.

## Get the required PS4 system files

Obliteration requires the `PS4UPDATE1.PUP.dec` firmware file in order to work. We can't provide these files due to legal reasons. You can use [pup_decrypt](https://github.com/idc/ps4-pup_decrypt) to get `PS4UPDATE1.PUP.dec` from `PS4UPDATE.PUP` using your PS4.

## Building from source

### Windows prerequisites

- Git
- Visual Studio 2019
  - `Desktop development with C++` workload is required
  - You can use 2022 but you need to install the `MSVC v142 - VS 2019 C++ x64/x86 build tools` component
- Rust 1.63+
- CMake 3.16+
  - Make sure you have `Add CMake to the system PATH` selected when installing

### Linux prerequisites

- Git
- GCC 9.4+
- Rust 1.63+
- CMake 3.16+

### macOS prerequisites

- macOS 12+
- Git
- Clang 13+
- Rust 1.63+
- CMake 3.16+


### Install Qt 6

You need to install Qt 6 on your system before you proceed. The minimum version is 6.2.

#### Windows specific requirements

You need `Qt Online Installer` for open-source to install Qt, downloaded from https://www.qt.io. The installer will ask you to sign in with a Qt account, which you can create for free. You need to check `Custom installation` and do not check `Qt for desktop development` that is using the MinGW toolchain. Make sure you have checked `MSVC 2019 64-bit` component in the `Select Components` page for the version you which to install and uncheck all of other components.

### Install Qt with Homebrew (macOS only)

```sh
brew install qt@6
```
### Open Qt Command Prompt (Windows only)

You should restart your computer before you proceed, to make sure all of environment variables that were updated from the previous steps are effective. Once restarted, open `Qt 6.X.X (MSVC 2019 64-bit)` from Start > Qt.

### Configure build system

Windows (Visual Studio 2019):

```bat
cmake -B build -A x64
```

Windows (Visual Studio 2022):

```bat
cmake -B build -A x64 -T v142
```

Linux and macOS:

```sh
cmake -B build -D CMAKE_BUILD_TYPE=Release
```

### Build

Windows:

```bat
cmake --build build --config Release
```

Linux and macOS:

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

- Don't be afraid to use `unsafe` when it is necessary. We are writing an application that requires very high performance code and we use Rust to assist us on this task, not to use Rust to compromise performance that C/C++ can provide.
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

### Development discussion

We have an IRC channel `#obliteration` on [OFTC](https://www.oftc.net) to discuss about the development. This channel is intended for discussion about the development and technical things only. You may get banned from the channel if you send other kind of messages that does not related to development.

## License

- All source code except `src/pfs` and `src/pkg` are licensed under MIT license.
- `src/pfs` and `src/pkg` licensed under LGPL-3.0 license.
- All release binaries are under GPL-3.0 license.
