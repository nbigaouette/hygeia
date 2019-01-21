# pycors

[![Build Status](https://travis-ci.org/nbigaouette/pycors.svg?branch=master)](https://travis-ci.org/nbigaouette/pycors)
[![Build status](https://ci.appveyor.com/api/projects/status/21n6gdcqj4oh68q8?svg=true&passingText=AppVeyor:%20%20passing)](https://ci.appveyor.com/project/nbigaouette/pycors)

`pycors` is a **Py**thon interpreter **co**ntroller built using **R**u**s**t. It's goal is to allow
individual projects to specify which interpreter to use using a `.python-version` file.

Python packaging situation is painful. macOS comes with Python 2.7 which is coming close to being
end-of-life. Additionally, it does not include `pip`, the package installer. `pip` was only included
by default with Python >= 3.4.

The [install instructions](https://pip.pypa.io/en/stable/installing/) for `pip` contains a large warning
against installing it in the system interpreter.

[`virtualenv`](https://virtualenv.pypa.io/) could be used, but it needs to be installed... using `pip`,
resulting in a chicken-and-egg situation.

`pycors` will download and compile specified versions of [Python](https://www.python.org/) and allow
switching between them easily.

The project took a lot of inspiration from [`pyenv`](https://github.com/pyenv/pyenv), which does something
similar. `pyenv` is written in Bash though, which
[prevents it from being used easily on Windows](https://github.com/pyenv/pyenv/issues/62).
`pycors` aims to be portable across Windows, Linux and macOS.


[![demo](https://asciinema.org/a/0K3KpPTPczFTdgSWyTJSjtpne.svg)](https://asciinema.org/a/0K3KpPTPczFTdgSWyTJSjtpne?autoplay=1)

## Requirements

Since Python interpreters are downloaded and compiled,
some build tools are required.

### macOS / OSX

Make sure dependencies are installed:

1. [Homebrew](https://brew.sh/)

    ```sh
    > brew install openssl xz
    ```
2. XCode

    ```sh
    > xcode-select --install
    ```

See the [Python Developer's Guide](https://devguide.python.org/setup/#macos-and-os-x) for more information.

### Linux

TBD

### Windows

TBD

## Usage

See `pycors --help` for all commands:

```sh
> pycors --help
pycors 0.0.7
Nicolas Bigaouette <nbigaouette@gmail.com>
Control which Python toolchain to use on a directory basis.

USAGE:
    pycors [SUBCOMMAND]

FLAGS:
    -h, --help
            Prints help information

    -V, --version
            Prints version information


SUBCOMMANDS:
    autocomplete    Print to stdout an autocomplete script for the specified shell
    help            Prints this message or the help of the given subcommand(s)
    install         Install version from `.python-version`
    list            List installed Python versions
    path            Get path to active interpreter
    run             Run a binary from the installed `.python-version`
    setup           Setup the shim
    use             Use specified Python versions
    version         Get version of active interpreter
```

### Initial Set Up

To set up pycors by installing it (and its shims) to `$PYCORS_HOME`
and configuring a bash shell:

```sh
> pycors setup bash
```

This will:

* Copy the `pycors` binary to `$PYCORS_HOME/shims/`;
* Create hard-links to it with Python binary names;
* Create a bash completion script in `$PYCORS_HOME/pycors.bash-completion`;
* Add `$PYCORS_HOME/shims/` to `$PATH` through `~/.bash_profile`;
* Add line sourcing $PYCORS_HOME/pycors.bash-completion in `~/.bash_profile`;

### Listing Interpreters

```sh
> pycors list
+--------+---------+------------------------------------------------+
| Active | Version | Location                                       |
+--------+---------+------------------------------------------------+
|        |  3.7.1  | /Users/nbigaouette/.pycors/installed/3.7.1/bin |
+--------+---------+------------------------------------------------+
|        |  3.7.2  | /Users/nbigaouette/.pycors/installed/3.7.2/bin |
+--------+---------+------------------------------------------------+
|        |  3.5.6  | /Users/nbigaouette/.pycors/installed/3.5.6/bin |
+--------+---------+------------------------------------------------+
|   ✓    |  3.6.8  | /Users/nbigaouette/.pycors/installed/3.6.8/bin |
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
> pycors list
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
> pycors path
/Users/nbigaouette/.pycors/installed/3.6.8/bin
```

To get the active interpreter's version:

```sh
> pycors version
3.6.8
```

### Set Interpreter as Active

This will create (or overwrite) the file `.python-version` (in the current working
directory) with the latest [_Semantic Versioning_](https://semver.org/) version
compatible with `3.7`.

```sh
> pycors select ~3.7
> pycors version
3.7.2
```

The parsing is performed by Rust's [semver crate](https://crates.io/crates/semver). For details
about the parsing, see the [_Requirements_](https://docs.rs/semver/0.9.0/semver/#requirements)
section in the [semver crate documentation](https://docs.rs/semver/0.9.0).

### Uninstall an Interpreter

Or simply delete the directory containing the installed interpreter, for example `$PYCORS_HOME/installed/3.5.6`
(where `$PYCORS_HOME` defaults to `$HOME/.pycors`).

Obtain the list of interpreters (and their installed path) using `pycors list`.

## Notes

### Logging

Export the `RUST_LOG` environment variable to enable pycors' log level:

```sh
> export RUST_LOG=pycors=debug
```

See the Rust crates [`log`](https://docs.rs/log) and [`env_logger`](https://docs.rs/env_logger) for
more information.

### Python Packages

Installing a Python package can be done using `pip` (which will call pycors' shim).

[numpy](http://www.numpy.org/):

```sh
> pip install numpy
```

[poetry](https://poetry.eustace.io/):

```sh
> pip install poetry
```

[pipenv](https://pipenv.readthedocs.io/en/latest/):

```sh
> pip install poetry

```
[pytest](https://docs.pytest.org/en/latest/):

```sh
> pip install pytest
```

#### Special Cases: Extra Executables

Unfortunately, pycors cannot detect if an executable was installed alongside the Python interpreter.
For example, installing an executable using `pip install <name>` will install 

Exceptions are:

* `pipenv`
* `poetry`
* `pytest`

for which pycors installs a shim automatically.

To execute any other executables, one can try to use `python` directly with the `-m` argument:

```sh
> python -m pipenv
```

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT)

at your option.
