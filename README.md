# Hygeia

[![Build Status](https://github.com/nbigaouette/hygeia/workflows/Build%20and%20Test/badge.svg?branch=master)](https://github.com/nbigaouette/hygeia/actions)
[![Security Audit](https://github.com/nbigaouette/hygeia/workflows/Nightly%20Security%20Audit/badge.svg?branch=master)](https://github.com/nbigaouette/hygeia/actions)
[![Coverage Status](https://codecov.io/gh/nbigaouette/hygeia/branch/master/graph/badge.svg)](https://codecov.io/gh/nbigaouette/hygeia)

<p align="center">
  <img src="logo.png" height="200" alt="logo"/>
</p>

_Hygeia_ is Python interpreter manager, built with ❤ in Rust. It's goal is to allow
individual projects to specify which interpreter to use using a `.python-version` file.

The previous project's name was _hygeia_.

Python packaging situation is painful. macOS comes with Python 2.7 which is coming close to being
end-of-life. Additionally, it does not include `pip`, the package installer. `pip` was only included
by default with Python >= 3.4.

The [install instructions](https://pip.pypa.io/en/stable/installing/) for `pip` contains a large warning
against installing it in the system interpreter.

[`virtualenv`](https://virtualenv.pypa.io/) could be used, but it needs to be installed... using `pip`,
resulting in a chicken-and-egg situation.

_Hygeia_ will download and compile specified versions of [Python](https://www.python.org/) and allow
switching between them easily.

The project took a lot of inspiration from [`pyenv`](https://github.com/pyenv/pyenv), which does something
similar. `pyenv` is written in Bash though, which
[prevents it from being used easily on Windows](https://github.com/pyenv/pyenv/issues/62).
Hygeia aims to be portable across Windows, Linux and macOS.


[![demo](https://asciinema.org/a/0K3KpPTPczFTdgSWyTJSjtpne.svg)](https://asciinema.org/a/0K3KpPTPczFTdgSWyTJSjtpne?autoplay=1)

## Requirements

Since Python interpreters are downloaded and compiled,
some build tools are required.

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

### Linux

TBD

### Windows

Nothing (expect Hygeia itself) is required to install a Python toolchain under Windows; pre-built
binaries are used.

## Installation

1. Visit the [release page](https://github.com/nbigaouette/hygeia/releases) to download the latest precompiled version for your platform (Linux, macOS, Windows).
2. Extract to a temporary location.
3. Open a terminal and execute `./hygeia setup bash`. This will:
    1. copy itself to `$HYGEIA_HOME` (`${HOME}/.hygeia`) as a shim for Python
    2. create the file `$HYGEIA_HOME/extra-packages-to-install.txt` containing
    [a list of Python packages to pip-install](extra-packages-to-install.txt)
    when flag `--extra`/`-e` is used with `install` or `select` commands
    3. setup `~/.bashrc` to add `${HOME}/.hygeia/shims` in the front of your `${PATH}`
4. You can delete the downloaded archive and the extracted binary.

## Compilation

As simple as `cargo build`!

## Usage

See `hygeia --help` for all commands:

```sh
❯ hygeia --help
hygeia 0.1.4
Nicolas Bigaouette <nbigaouette@gmail.com>
Control which Python toolchain to use on a directory basis.

USAGE:
    hygeia [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    autocomplete    Print to stdout an autocomplete script for the specified shell
    help            Prints this message or the help of the given subcommand(s)
    install         Install version, either from the provided version or from `.python-version`
    list            List installed Python versions
    path            Get path to active interpreter
    run             Run a binary from the installed `.python-version`
    select          Select specified Python versions to use
    setup           Setup the shim
    version         Get version of active interpreter
```

### Initial Set Up

To set up Hygeia by installing it (and its shims) to `$HYGEIA_HOME`
and configuring a bash shell:

```sh
❯ hygeia setup bash
```

This will:

* Copy the `hygeia` binary to `$HYGEIA_HOME/shims/`;
* Create hard-links to it with Python binary names;
* Create a bash completion script in `$HYGEIA_HOME/hygeia.bash-completion`;
* Add `$HYGEIA_HOME/shims/` to `$PATH` through `~/.bashrc`;
* Add line sourcing `$HYGEIA_HOME/hygeia.bash-completion` in `~/.bashrc`;
* Create the file `$HYGEIA_HOME/extra-packages-to-install.txt` containing
  [a list of Python packages to pip-install](extra-packages-to-install.txt)
  when flag `--extra`/`-e` is used with `install` or `select` command.

### Listing Interpreters

```sh
❯ hygeia list
+--------+---------+------------------------------------------------+
| Active | Version | Location                                       |
+--------+---------+------------------------------------------------+
|        |  3.7.1  | /Users/nbigaouette/.hygeia/installed/3.7.1/bin |
+--------+---------+------------------------------------------------+
|        |  3.7.2  | /Users/nbigaouette/.hygeia/installed/3.7.2/bin |
+--------+---------+------------------------------------------------+
|        |  3.5.6  | /Users/nbigaouette/.hygeia/installed/3.5.6/bin |
+--------+---------+------------------------------------------------+
|   ✓    |  3.6.8  | /Users/nbigaouette/.hygeia/installed/3.6.8/bin |
+--------+---------+------------------------------------------------+
|        |  3.7.2  | /usr/local/bin                                 |
+--------+---------+------------------------------------------------+
|        | 2.7.15  | /usr/local/bin                                 |
+--------+---------+------------------------------------------------+
|        | 2.7.10  | /usr/bin                                       |
+--------+---------+------------------------------------------------+
```

If the file `.python-version` contains a version _not_ installed, the list
reports it as active but not available:

```sh
❯ hygeia list
+--------+---------+------------------------------------------------+
| Active | Version | Location                                       |
+--------+---------+------------------------------------------------+
|   ✗    | = 3.6.8 |                 Not installed                  |
+--------+---------+------------------------------------------------+
|        |  3.7.2  | /usr/local/bin                                 |
+--------+---------+------------------------------------------------+
|        | 2.7.15  | /usr/local/bin                                 |
+--------+---------+------------------------------------------------+
|        | 2.7.10  | /usr/bin                                       |
+--------+---------+------------------------------------------------+
```

To get the active interpreter's path:

```sh
❯ hygeia path
/Users/nbigaouette/.hygeia/installed/3.6.8/bin
```

To get the active interpreter's version:

```sh
❯ hygeia version
3.6.8
```

### Set Interpreter as Active

This will create (or overwrite) the file `.python-version` (in the current working
directory) with the latest [_Semantic Versioning_](https://semver.org/) version
compatible with `3.7`.

```sh
❯ hygeia select ~3.7
❯ hygeia version
3.7.2
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

Or simply delete the directory containing the installed interpreter, for example `$HYGEIA_HOME/installed/3.5.6`
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

### Python Packages

Installing a Python package can be done using `pip` (which will call hygeia' shim).

[numpy](http://www.numpy.org/):

```sh
❯ pip install numpy
```

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
