[![Build Status](https://dev.azure.com/peter-bulk/Misc/_apis/build/status/pkgw.rubbl?branchName=master)](https://dev.azure.com/peter-bulk/Misc/_build/latest?definitionId=9&branchName=master)
![rubbl](https://img.shields.io/crates/v/rubbl?label=rubbl)
![rubbl_casatables](https://img.shields.io/crates/v/rubbl_casatables?label=rubbl_casatables)
![rubbl_casatables_impl](https://img.shields.io/crates/v/rubbl_casatables_impl?label=rubbl_casatables_impl)
![rubbl_core](https://img.shields.io/crates/v/rubbl_core?label=rubbl_core)

<!--
![rubbl_fits](https://img.shields.io/crates/v/rubbl_fits?label=rubbl_fits)
![rubbl_miriad](https://img.shields.io/crates/v/rubbl_miriad?label=rubbl_miriad)
![rubbl_visdata](https://img.shields.io/crates/v/rubbl_visdata?label=rubbl_visdata)
-->

# rubbl

*Rust + Hubble = rubbl = astrophysics in Rust*.

This is an exploratory set of basic libraries aimed at allowing astrophysical
software, especially astronomy data processing software, to be written in the
[Rust] language.

[Rust]: https://www.rust-lang.org/

The current star of of the show is the `rubbl_casacore` crate, which provides
access to the “table” file format used by the [CASA] radio astronomy data
processing suite. The C++ implementation of the table format is wrapped with
an ergonomic-as-possible Rust interface.

[CASA]: https://casa.nrao.edu/

## Installation

The way that Rust packaging works, you probably don’t need to install these
crates on their own. Instead, you should create your own crate that specifies
these core crates as dependencies. See [rubbl-rxpackage] for an example of
this workflow.

[rubbl-rxpackage]: https://github.com/pkgw/rubbl-rxpackage

However, this repository does contain a few runnable example tools. Compiling
the code requires toolchains for the C++ and Rust languages. Installing the
Rust toolchain is generally straightforward; see [the Rust installation page]
for instructions. If the toolchains are properly installed, you can compile
and run an example program that summarizes the contents of a CASA table data
set by checking out this repository and running:

[the Rust installation page]: https://www.rust-lang.org/tools/install

```
cargo run --example tableinfo -- path/to/my/table.ms
```

Finally, this repository does contain a core command-line program called
`rubbl` that simply dispatches invocations to other commands in the same
fashion as `git` and `cargo`: `rubbl foo --bar` is farmed out by running the
command `rubbl-foo --bar`. This tool can be installed without even needing to
check out this repository — run the command:

```
cargo install rubbl
```

## Legalities

The bulk of the code is licensed under the MIT License. The `casatables_impl`
crate contains code copied from
[casacore](https://github.com/casacore/casacore), which is licensed under the
GNU Library General Public License version 2, or any subsequent version at your
discretion. In turn, it contains code copied from [dysco](https://github.com/aroffringa/dysco), which is licensed under the GNU Library General Public Licens version 3.
