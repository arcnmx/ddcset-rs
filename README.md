# ddcset

[![travis-badge][]][travis] [![release-badge][]][cargo] [![license-badge][]][license]

`ddcset` is an application for controlling connected monitors over DDC/CI.


## Platforms

Currently supported platforms:

- Linux
  - `i2c-dev`: requires a supported graphics driver, and `modprobe i2c-dev`
    - Modern and open source drivers should support this. Older proprietary
      drivers such as fglrx may not work.
    - [NVIDIA GPUs will require additional configuration to work](#nvidia-drivers-on-linux).
- Windows
  - Windows Monitor Configuration API provides limited DDC/CI support on all GPUs.
  - NVIDIA NVAPI provides improved DDC/CI support for supported GPUs.

## Installation

[Binaries are available on some platforms](https://github.com/arcnmx/mccs-rs/releases).
[Cargo](https://www.rust-lang.org/en-US/install.html) can also be used to install
directly from source:

    cargo install --force ddcset


## NVIDIA drivers on Linux

The NVIDIA Linux drivers have had broken DDC/CI support for years now. [There are
workarounds](http://www.ddcutil.com/nvidia/) but it seems that it is not
currently possible to use DDC/CI over DisplayPort.

[travis-badge]: https://img.shields.io/travis/arcnmx/ddcset-rs/master.svg?style=flat-square
[travis]: https://travis-ci.org/arcnmx/ddcset-rs
[release-badge]: https://img.shields.io/crates/v/ddcset.svg?style=flat-square
[cargo]: https://crates.io/crates/ddcset
[license-badge]: https://img.shields.io/badge/license-MIT-ff69b4.svg?style=flat-square
[license]: https://github.com/arcnmx/ddcset-rs/blob/master/COPYING
