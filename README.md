# Obliteration
[![CI](https://github.com/ultimaweapon/obliteration/actions/workflows/ci.yml/badge.svg)](https://github.com/ultimaweapon/obliteration/actions/workflows/ci.yml)

Obliteration is an experimental PS4 emulator based on [Kyty](https://github.com/InoriRus/Kyty). The project is in the process of migrating source from Kyty to make it work on both Windows and Linux so it cannot run any games that Kyty able to run yet.

## Screen shots

![Game list](screenshots/game-list.png)

## Features

- Built-in PKG file supports.

## System requirements

- Windows 10 x64 or Linux x86-64.
- CPU that supports all of instructions on [PS4 CPU](https://en.wikipedia.org/wiki/Jaguar_(microarchitecture)#Instruction_set_support). All modern Intel/AMD CPUs should meet with this requirement.

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

Remove `build` directory from the installation from source step and open this folder with VS Code. Everything should work out of the box (e.g. code completion, debugging, etc).

### Get a homebrew application for testing

If you don't have a PS4 application for testing you can download PS Scene Quiz for free [here](https://pkg-zone.com/details/LAPY10010).

### Rules for Rust sources

- Don't be afraid to use `unsafe` when it is necessary. We are written an application that required very high performance code and we use Rust to assist us on this task, not to use Rust to compromise performance that C/C++ can provides.
- Any functions that operate on pointers don't need to mark as `unsafe`. The reasons is because it will required the caller to wrap it in `unsafe`. We already embrace `unsafe` code so no point to make it harder to use.
- Don't chain method calls without intermidate variable if the result code is hard to follow. We encourage code readability than a pleasure when writing so try to make it easy to read and understand for other people.

### Rules for C++ sources

Just follow how Qt is written (e.g. coding style, etc.). Always prefers Qt classes over `std` when possible so you don't need to handle exception. Do not use Qt `ui` file to design the UI because it will break on high-DPI screen.

### Starting point

The entry point of the application is inside `src/main.cpp`.

## License

- All source code except `src/pfs` and `src/pkg` are licensed under MIT license.
- `src/pfs` and `src/pkg` licensed under LGPL-3.0 license.
- All release binaries are under GPL-3.0 license.
