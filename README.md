<p align="center"><img alt="Logo" src="logo.png"></p>

## About
[![CI](https://github.com/obhq/obliteration/actions/workflows/main.yml/badge.svg)](https://github.com/obhq/obliteration/actions/workflows/main.yml)
[![Zulip](https://img.shields.io/badge/zulip-join_chat-brightgreen.svg)](https://obkrnl.zulipchat.com)

Obliteration is a free and open-source PlayStation 4 kernel rewritten in Rust. Our goal is to run the PlayStation 4 system software on Windows, Linux and macOS using a custom made virtualization stack optimized specifically for Obliteration. **The project is under development and cannot run any games yet**. The reason it take so long is because we decided to go with the correct path without stubbing as much as possible.

This project started as a hard-fork from [Kyty](https://github.com/InoriRus/Kyty). Then we decided to rewrite the whole project from scratch by using Kyty and [Uplift](https://github.com/idc/uplift) as a reference to help us getting started with the project.

The project logo and icon was designed by [VocalFan](https://github.com/VocalFan).

## Get a daily build

Please note that we cannot run any games yet as stated on the above. But if you want to try or help on testing you can download binaries from the latest commits [here](https://github.com/obhq/obliteration/actions/workflows/main.yml). You **MUST** sign in to GitHub otherwise you will not be able to download files.

## Building and Development

Information related to Obliteration development and building from source can be found on our [developer documentation](https://dev.obliteration.net).

### UI Icons

We use icons from https://materialdesignicons.com for UI.

## License

- `src/ansi_escape.hpp`, `src/ansi_escape.cpp`, `src/log_formatter.hpp` and `src/log_formatter.cpp` are licensed under GPL-3.0 only.
- `src/param`, `src/pfs` and `src/pkg` are licensed under LGPL-3.0.
- All other source code are licensed under either MIT License or Apache License, Version 2.0; or both.
- All release binaries are under GPL-3.0.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Obliteration by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
