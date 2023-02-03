<p align="center"><img alt="Logo" src="logo.png"></p>

## About
[![CI](https://github.com/obhq/obliteration/actions/workflows/main.yml/badge.svg)](https://github.com/obhq/obliteration/actions/workflows/main.yml) [![Matrix](https://img.shields.io/matrix/obliteration:matrix.org?color=740d03&label=Obliteration&logo=matrix)](https://matrix.to/#/#obliteration:matrix.org)

Obliteration is an experimental PS4 emulator using [Kyty](https://github.com/InoriRus/Kyty) and [Uplift](https://github.com/idc/uplift) as a reference. The project is under development and cannot run any games that Kyty is able to run yet.

**The original author of Kyty is not involved in the development of Obliteration in anyway.** Obliteration is a completely separated project. The reason you see the author of Kyty in the contributor list is because this project contains commits from Kyty.

## Get a daily build

You can download the binaries from the latest commits [here](https://github.com/obhq/obliteration/actions/workflows/main.yml). You **MUST** sign-in to GitHub otherwise you will not be able to download the file.

## Screenshots

![Game list](screenshots/game-list.png)

Thanks [Mou-Ikkai](https://github.com/Mou-Ikkai) for the awesome icon!

## Obliteration discussion

We have a Matrix Room Space `#obliteration:matrix.org` on [Matrix.to](https://matrix.to/#/#obliteration:matrix.org) to discuss about the project. Read each room's Topic for more information.

## Features

- [ ] Built-in PUP file supports for decrypted PUP from [pup_decrypt](https://github.com/idc/ps4-pup_decrypt).
- [x] Built-in PKG file supports for Fake PKG.
- [x] Game library.
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

### Linux specific requirements

Obliteration supports only 4KB/8KB/16KB page. Most people should not have any problem with this because 4KB is the default for most distros.

## Get the required PS4 system files

Obliteration requires the `PS4UPDATE1.PUP.dec` firmware file in order to work. We can't provide these files due to legal reasons. You can use [pup_decrypt](https://github.com/idc/ps4-pup_decrypt) to get `PS4UPDATE1.PUP.dec` from `PS4UPDATE.PUP` using your PS4.

## Building from source

### Windows prerequisites

- Visual Studio 2022
  - `Desktop development with C++` workload is required
  - You need to install the `MSVC v142 - VS 2019 C++ x64/x86 build tools` component
- Rust on latest stable channel
- CMake 3.21+
  - Make sure you have `Add CMake to the system PATH` selected when installing

### Linux prerequisites

- GCC 9.4+
- Rust on latest stable channel
- CMake 3.21+

### macOS prerequisites

- macOS 12+
- Homebrew
- Clang 13+
- Rust on latest stable channel
- CMake 3.21+

### Install Qt 6

You need to install Qt 6 on your system before you proceed. The minimum version is 6.2.

#### Windows specific requirements

You need `Qt Online Installer` for open-source to install Qt, downloaded from https://www.qt.io. The installer will ask you to sign in with a Qt account, which you can create for free. You need to check `Custom installation` and do not check `Qt for desktop development` that is using the MinGW toolchain. Make sure you have checked `MSVC 2019 64-bit` component in the `Select Components` page for the version you wish to install and uncheck all of other components.

Once installation is completed you need to set the `CMAKE_PREFIX_PATH` environment variable to the full path of the installed version (e.g. `C:\Qt\6.2.4\msvc2019_64`). To set an environment variable:

1. Open a run dialog with <kbd>Win</kbd> + <kbd>R</kbd>.
2. Enter `sysdm.cpl` then click `OK`.
3. Go to `Advanced` tab then click on `Environment Variables...`.
4. Click `New...` to create a new environment variable. Just create for either `User variables` or `System variables`, not both.

Then restart your computer to make it effective.

#### Install Qt with Homebrew (macOS only)

```sh
brew install qt@6
```

### Configure build system

```sh
cmake --preset PRESET .
```

The value of `PRESET` will be depend on your platform and the build configuration you want to use. Current available presets are:

- windows-release
- windows-debug
- linux-release
- linux-debug
- mac-release
- mac-debug

Choose `[YOUR-PLATFORM]-release` for optimized outputs. The `*-debug` is designed for development only and the outputs will not optimized.

### Build

```sh
cmake --build --preset PRESET
```

## Development

We recommended Visual Studio Code as a code editor with the following extensions:

- [C/C++](https://marketplace.visualstudio.com/items?itemName=ms-vscode.cpptools)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [CMake](https://marketplace.visualstudio.com/items?itemName=twxs.cmake)
- [CMake Tools](https://marketplace.visualstudio.com/items?itemName=ms-vscode.cmake-tools)
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)

Remove `build` directory from the previous step and open this folder with VS Code. It will ask which CMake preset to use and you need to choose the debug version (e.g. `windows-debug`). Everything should work out of the box (e.g. code completion, debugging, etc).

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

## License

- All source code except `src/pfs` and `src/pkg` are licensed under MIT license.
- `src/pfs` and `src/pkg` licensed under LGPL-3.0 license.
- All release binaries are under GPL-3.0 license.
