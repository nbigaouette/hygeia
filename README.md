# Hygeia

[![Build Status](https://github.com/nbigaouette/hygeia/workflows/Build%20and%20Test/badge.svg?branch=master)](https://github.com/nbigaouette/hygeia/actions)
[![Security Audit](https://github.com/nbigaouette/hygeia/workflows/Nightly%20Security%20Audit/badge.svg?branch=master)](https://github.com/nbigaouette/hygeia/actions)
[![Coverage Status](https://codecov.io/gh/nbigaouette/hygeia/branch/master/graph/badge.svg)](https://codecov.io/gh/nbigaouette/hygeia)

<p align="center">
  <img src="logo.png" height="200" alt="logo"/>
</p>

_Hygeia_ is Python interpreter manager, built with ❤ in Rust. It's goal is to allow
individual projects to specify which interpreter to use using a `.python-version` file.

The previous project's name was _pycors_.

- [Hygeia](#hygeia)
  - [Installation](#installation)
    - [Requirements](#requirements)
    - [macOS / OSX](#macos--osx)
    - [Linux](#linux)
      - [Deb-based](#deb-based)
      - [Yum-based](#yum-based)
      - [DNF-based](#dnf-based)
      - [Pacman-based](#pacman-based)
    - [Windows](#windows)
    - [Easy Installation](#easy-installation)
    - [Manual Installation](#manual-installation)
    - [Compilation](#compilation)
  - [Project Details](#project-details)
  - [Usage](#usage)
    - [Installing a Python Toolchain](#installing-a-python-toolchain)
    - [Listing Interpreters](#listing-interpreters)
    - [Set Interpreter as Active](#set-interpreter-as-active)
    - [Uninstall an Interpreter](#uninstall-an-interpreter)
  - [Notes](#notes)
    - [Logging](#logging)
  - [License](#license)
  - [Conduct](#conduct)

## Installation

### Requirements

Since Python interpreters are downloaded and compiled,
some build tools are required.

<details>
<summary>macOS / OSX</summary>

### macOS / OSX

Make sure dependencies are installed:

1. [Homebrew](https://brew.sh/)

    ```sh
    ❯ brew install openssl readline sqlite3 xz zlib
    ```
2. XCode

    ```sh
    ❯ xcode-select --install
    ```

See the [Python Developer's Guide](https://devguide.python.org/setup/#macos-and-os-x) for more information.

</details>

<details>
<summary>Linux</summary>

### Linux

Please refer to [pyenv](https://github.com/pyenv/pyenv/wiki#suggested-build-environment)'s wiki for more details.

#### Deb-based

```sh
❯ sudo apt-get update; sudo apt-get install --no-install-recommends make build-essential libssl-dev zlib1g-dev libbz2-dev libreadline-dev libsqlite3-dev wget curl llvm libncurses5-dev xz-utils tk-dev libxml2-dev libxmlsec1-dev libffi-dev liblzma-dev
```

#### Yum-based

```sh
❯ yum install gcc zlib-devel bzip2 bzip2-devel readline-devel sqlite sqlite-devel openssl-devel tk-devel libffi-devel xz
```

#### DNF-based

```sh
❯ dnf install make gcc zlib-devel bzip2 bzip2-devel readline-devel sqlite sqlite-devel openssl-devel tk-devel libffi-devel xz
```

#### Pacman-based

```sh
pacman -S base-devel openssl zlib
```

</details>

<details>
<summary>Windows</summary>

### Windows

Nothing (expect Hygeia itself) is required to install a Python toolchain under Windows; pre-built
binaries are used.

</details>

### Easy Installation

Copy-paste and run the following in a terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/nbigaouette/hygeia/master/install.sh | sh
```

### Manual Installation

If you are not comfortable with running the curl installation above, simply follow these steps:

1. Visit the [release page](https://github.com/nbigaouette/hygeia/releases) to download the latest precompiled version for your platform (Linux, macOS, Windows).
2. Extract to a temporary location.
3. Open a terminal and execute `./hygeia setup <SHELL>` (where `SHELL` is one of `bash`, `zsh` or `powershell`).
4. You can delete the downloaded archive and the extracted binary.

### Compilation

As simple as `cargo build`! Then, to install and configure your shell:

```sh
cargo run -- setup <SHELL>
```

where `<SHELL>` is to be replaced with `bash`, `zsh` or `powershell`.

## Project Details

Python packaging situation is painful. Having been spoiled by [rustup](https://rustup.rs/), Rust's
installer and toolchain manager, I wanted a similar experience for Python.

_Hygeia_ will download, compile and manage different versions of [Python](https://www.python.org/).
Projects can then specify which versions they want to use through a `.python-version` file,
similarly to the [`rust-toolchain`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
the rustup uses.
This allows different project to use different Python versions without messing up one's system installation.

The project took a lot of inspiration from [`pyenv`](https://github.com/pyenv/pyenv), which does something
similar. `pyenv` is written in Bash though, which
[prevents it from being used easily on Windows](https://github.com/pyenv/pyenv/issues/62).
Hygeia aims to be portable across Windows, Linux and macOS.

[![demo](https://asciinema.org/a/0K3KpPTPczFTdgSWyTJSjtpne.svg)](https://asciinema.org/a/0K3KpPTPczFTdgSWyTJSjtpne?autoplay=1)

## Usage

See `hygeia --help` for all commands:

```sh
❯ hygeia --help
hygeia v0.3.3 (1f6c49f02 2020-02-03)
Control which Python toolchain to use on a directory basis

USAGE:
    hygeia [FLAGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode (-v, -vv, -vvv, etc.)

SUBCOMMANDS:
    help       Prints this message or the help of the given subcommand(s)
    install    Install version, either from the provided version or from '.python-version'
    list       List installed Python versions
    path       Get path to active interpreter
    run        Run a binary from the installed '.python-version'
    select     Select specified Python versions to use
    setup      Setup the shim
    update     Update pycors to latest version
    version    Get version of active interpreter
```

### Installing a Python Toolchain

Install the latest semver compatible version:

```sh
❯ hygeia install --extra 3.9
```

which is equivalent to:

```sh
❯ hygeia install --extra"~3.9"
```

**NOTE**: Zsh requires quoting the version, while bash does not.

Install a specific version:

```sh
❯ hygeia install "=3.8.5"
```

### Listing Interpreters

```sh
❯ cat .python-version
= 3.8.2
❯ hygeia list
+--------+---------+---------------------+--------------------------------------------------------+
| Active | Version | Installed by hygeia | Location                                               |
+--------+---------+---------------------+--------------------------------------------------------+
|        |  3.8.6  |                     | /usr/local/bin                                         |
+--------+---------+---------------------+--------------------------------------------------------+
|   ✓    |  3.8.2  |          ✓          | /Users/nbigaouette/.hygeia/installed/cpython/3.8.2/bin |
+--------+---------+---------------------+--------------------------------------------------------+
|        |  3.8.2  |                     | /usr/bin                                               |
+--------+---------+---------------------+--------------------------------------------------------+
|        |  3.8.1  |          ✓          | /Users/nbigaouette/.hygeia/installed/cpython/3.8.1/bin |
+--------+---------+---------------------+--------------------------------------------------------+
|        | 2.7.16  |                     | /usr/bin                                               |
+--------+---------+---------------------+--------------------------------------------------------+
```

If the file `.python-version` contains a version _not_ installed, the list
reports it as active but not available:

```sh
❯ cat .python-version
= 3.8.3
❯ hygeia list
+--------+---------+---------------------+--------------------------------------------------------+
| Active | Version | Installed by hygeia | Location                                               |
+--------+---------+---------------------+--------------------------------------------------------+
|   ✗    |  3.8.3  |                     |                                                        |
+--------+---------+---------------------+--------------------------------------------------------+
|        |  3.8.6  |                     | /usr/local/bin                                         |
+--------+---------+---------------------+--------------------------------------------------------+
|        |  3.8.2  |          ✓          | /Users/nbigaouette/.hygeia/installed/cpython/3.8.2/bin |
+--------+---------+---------------------+--------------------------------------------------------+
|        |  3.8.2  |                     | /usr/bin                                               |
+--------+---------+---------------------+--------------------------------------------------------+
|        |  3.8.1  |          ✓          | /Users/nbigaouette/.hygeia/installed/cpython/3.8.1/bin |
+--------+---------+---------------------+--------------------------------------------------------+
|        | 2.7.16  |                     | /usr/bin                                               |
+--------+---------+---------------------+--------------------------------------------------------+
```

To get the active interpreter's path:

```sh
❯ hygeia path
/Users/nbigaouette/.hygeia/installed/cpython/3.8.2/bin
```

To get the active interpreter's version:

```sh
❯ hygeia version
3.8.2
```

### Set Interpreter as Active

This will create (or overwrite) the file `.python-version` (in the current working
directory) with the latest [_Semantic Versioning_](https://semver.org/) version
compatible with `3.7`.

```sh
❯ hygeia select "~3.7"
❯ hygeia version
3.7.9
```

Note that `--extra` can be used with `select` or `install` command to read file
`$HYGEIA_HOME/extra-packages-to-install.txt` and `pip install` all packages specified.
Additionally, `--extra-from` can also be used to specify a different file. Both flags
can be used at the same time and the content of both files will be used.
Lines starting with `#` are ignored (as comments).

The parsing is performed by Rust's [semver crate](https://crates.io/crates/semver). For details
about the parsing, see the [_Requirements_](https://docs.rs/semver/0.9.0/semver/#requirements)
section in the [semver crate documentation](https://docs.rs/semver/0.9.0).

### Uninstall an Interpreter

Simply delete the directory containing the installed interpreter, for example `$HYGEIA_HOME/installed/cpython/3.8.2`
(where `$HYGEIA_HOME` defaults to `$HOME/.hygeia`).

Obtain the list of interpreters (and their installed path) using `hygeia list`.

## Notes

### Logging

Export the `RUST_LOG` environment variable to enable hygeia' log level:

```sh
❯ export RUST_LOG=hygeia=debug
```

See the Rust crates [`log`](https://docs.rs/log) and [`env_logger`](https://docs.rs/env_logger) for
more information.

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT)

at your option.

## Conduct

The [Rust Code of Conduct](https://www.rust-lang.org/conduct.html) shall be respected. For
escalation or moderation issues please contact Nicolas (nbigaouette@gmail.com)
instead of the Rust moderation team.
