# Setup

In order to build Obliteration from source make sure you have the following prerequisites.

## All platforms

- [Rust on the latest stable channel](https://www.rust-lang.org/tools/install)
  - Make sure you install using `rustup`
  - On Linux your distro may provide a package for this
  - On macOS you can install from Homebrew
- [CMake 3.21+](https://cmake.org/download)
  - On Windows make sure you have `Add CMake to the system PATH` selected when installing
  - On Linux it is likely your distro already provided a package for this
  - On macOS you can install from Homebrew

## Windows

- [Visual Studio 2022](https://visualstudio.microsoft.com/vs)
  - Rust installer should already install this for you so you should not need to install this manually
  - Community edition are free for open-source project
  - `Desktop development with C++` workload is required
- [Ninja](https://ninja-build.org)
  - You can install from [Chocolatey](https://chocolatey.org/install) with `choco install ninja`
  - If you install via the other method make sure Ninja is added to `PATH` environment variable

## Linux

- GCC that supports C++17
- GNU Make

## macOS

- Xcode Command Line Tools
