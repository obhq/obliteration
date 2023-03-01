<p align="center"><img alt="Logo" src="logo.png"></p>

## About
[![CI](https://github.com/obhq/obliteration/actions/workflows/main.yml/badge.svg)](https://github.com/obhq/obliteration/actions/workflows/main.yml) [![Matrix](https://img.shields.io/matrix/obliteration:matrix.org?color=740d03&label=Obliteration&logo=matrix)](https://matrix.to/#/#obliteration:matrix.org)

Obliteration is an experimental PS4 emulator using [Kyty](https://github.com/InoriRus/Kyty) and [Uplift](https://github.com/idc/uplift) as a reference. The project is under development and cannot run any games that Kyty is able to run yet.

**The original author of Kyty is not involved in the development of Obliteration in any way.** Obliteration is a completely separate project. The reason you see the author of Kyty in the contributor list is that this project contains commits from Kyty.

## Get a daily build

You can download binaries from the latest commits [here](https://github.com/obhq/obliteration/actions/workflows/main.yml). You **MUST** sign in to GitHub otherwise you will not be able to download the file.

## Screenshots

![Game list](screenshots/game-list.png)

Thanks [Mou-Ikkai](https://github.com/Mou-Ikkai) for the awesome icon!

## Obliteration discussion

We have a Matrix Room Space `#obliteration:matrix.org` on [Matrix.to](https://matrix.to/#/#obliteration:matrix.org) to discuss the project. Read each room's Topic for more information.

## Features

- [x] Cross-platform with native binary for each platform.
- [x] Supports both x86-64 and AArch64 CPU.
- [x] Built-in FTP client to pull the decrypted firmware from jailbroken PS4.
- [x] Built-in PKG file supports for Fake PKG.
- [x] Game library.
- [ ] Emulate system calls instead of user-space libraries.

## System requirements

- Windows 10, Linux and macOS.
- x86-64 or AArch64 CPU.
- A jailbroken PS4 with FTP server that support SELF decryption.

### Windows-specific requirements

- [Microsoft Visual C++ 2022 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). If there is an error related to `msvcp140.dll`, `vcruntime140.dll`, or `vcruntime140_1.dll` that means you need to install this manually. It's likely your system already has it, so try to run Obliteration first.

### Linux-specific requirements

Obliteration supports only 4KB/8KB/16KB pages. Most people should not have any problem with this because 4KB is the default for most distros.

## Building from source

### Windows prerequisites

- Visual Studio 2022
  - `Desktop development with C++` workload is required
- Rust on the latest stable channel
- CMake 3.24+
  - Make sure you have `Add CMake to the system PATH` selected when installing

### Linux prerequisites

- GCC 9.4+
- Rust on the latest stable channel
- CMake 3.24+

### macOS prerequisites

- macOS 12+
- Homebrew
- Clang 13+
- Rust on the latest stable channel
- CMake 3.24+

### Install Qt 6

You need to install Qt 6 on your system before you proceed. The minimum version is 6.2.

#### Windows-specific requirements

You need `Qt Online Installer` for open-source to install Qt, downloaded from https://www.qt.io. The installer will ask you to sign in with a Qt account, which you can create for free. You need to check `Custom installation` and do not check `Qt for desktop development` that is using the MinGW toolchain. Make sure you have checked the `MSVC 2019 64-bit` component in the `Select Components` page for the version you wish to install and uncheck all of the other components.

Once installation is completed you need to set the `CMAKE_PREFIX_PATH` environment variable to the full path of the installed version (e.g. `C:\Qt\6.2.4\msvc2019_64`). To set an environment variable:

1. Open a run dialog with <kbd>Win</kbd> + <kbd>R</kbd>.
2. Enter `sysdm.cpl` then click `OK`.
3. Go to the `Advanced` tab then click on `Environment Variables...`.
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

The value of `PRESET` will depend on your platform and the build configuration you want to use. The current available presets are:

- windows-release
- windows-debug
- linux-release
- linux-debug
- mac-release
- mac-debug

Choose `[YOUR-PLATFORM]-release` for optimized outputs. The `*-debug` is designed for development only and the outputs will not be optimized.

### Build

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

Remove the `build` directory from the previous step and open this folder with VS Code. It will ask which CMake preset to use and you need to choose the debug version (e.g. `windows-debug`). Everything should work out of the box (e.g. code completion, debugging, etc).

### Get a homebrew application for testing

If you don't have a PS4 application for testing you can download PS Scene Quiz for free [here](https://pkg-zone.com/details/LAPY10010).

### Rules for Rust sources

- Use unsafe code only when you know what you are doing. When you do try to wrap it in a safe function so other people who are not familiar with unsafe code can have a safe life.
- Don't chain method calls without an intermediate variable if the result code is hard to follow. We encourage code readability as a pleasure when writing so try to make it easy to read and understand for other people.
- Do not blindly cast an integer. Make sure the value can fit in a destination type. We don't have any plans to support non-64-bit systems so the pointer size and its related types like `usize` are always 64-bits.

### Rules for C++ sources

Just follow how Qt is written (e.g. coding style, etc.). Always prefers Qt classes over `std` when possible so you don't need to handle exceptions. Do not use the Qt `ui` file to design the UI because it will break on a high-DPI screen.

### Starting point

The application consists of 2 binaries:

1. Main application. This is what users will see when they launch Obliteration. Its entry point is inside `src/main.cpp`.
2. Emulator kernel. This is where PS4 emulation takes place. Its entry point is inside `src/kernel/src/main.rs`.

### Debugging kernel

Create `.kernel-debug` in the root of the repository. The content of this file is YAML and the kernel will deserialize it to the `Args` struct in `src/kernel/src/main.rs` when passing the `--debug` flag to the kernel. We already provided a launch configuration for VS Code so all you need to do is just choose `Kernel` as a configuration and start debugging.

### Action icons

We use icons from https://materialdesignicons.com for action icons (e.g. on the menu and toolbar).

## License

- `src/ansi_escape.hpp`, `src/ansi_escape.cpp`, `src/log_formatter.hpp` and `src/log_formatter.cpp` are licensed under GPL-3.0 only.
- `src/pfs` and `src/pkg` are licensed under LGPL-3.0 license.
- All other source code is licensed under MIT license.
- All release binaries are under GPL-3.0 license.
