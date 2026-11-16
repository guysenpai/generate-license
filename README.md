# generate-license

`generate-license` writes the license files of a project from SPDX
identifiers: `generate-license MIT` creates `LICENSE`, and
`generate-license MIT OR Apache-2.0` creates `LICENSE-MIT` and
`LICENSE-APACHE`. The license texts are embedded in the binary, so it works
offline. It fills in the copyright line with the year and your name, names
the files so that GitHub recognizes the license, and never overwrites a file
unless you ask it to.

## Installation

Linux and macOS:

```
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/guysenpai/generate-license/releases/latest/download/generate-license-installer.sh | sh
```

Windows:

```
powershell -ExecutionPolicy Bypass -c "irm https://github.com/guysenpai/generate-license/releases/latest/download/generate-license-installer.ps1 | iex"
```

From source, with a Rust toolchain:

```
cargo install --git https://github.com/guysenpai/generate-license
```

The installers place the binary in `$XDG_BIN_HOME` if it is set, and in
`~/.local/bin` otherwise.

## Usage

```
Generate license files from SPDX identifiers

Usage: generate-license [OPTIONS] <EXPRESSION>...
       generate-license --list

Arguments:
  <EXPRESSION>...  SPDX identifiers or expression, e.g. MIT OR Apache-2.0

Options:
      --year <YEAR>      Copyright year or range (default: current year)
      --no-year          Omit the year from the copyright line
      --holder <HOLDER>  Copyright holder (default: git config user.name)
  -o, --output <PATH>    Output path, "-" for stdout (single license only)
  -f, --force            Overwrite existing files
      --list             List supported licenses
  -h, --help             Print help
  -V, --version          Print version
```

### Expressions

Identifiers are matched without regard to case and written back in their
canonical form. `AND`, `OR`, parentheses and `/` only separate identifiers:
`MIT OR Apache-2.0`, `MIT AND Apache-2.0` and `MIT/Apache-2.0` write the same
files. An identifier named twice is written once.

The deprecated identifiers `GPL-2.0`, `GPL-3.0`, `LGPL-2.1`, `LGPL-3.0` and
`AGPL-3.0` are accepted with a warning. They resolve to the `-only` variant,
or to the `-or-later` variant when followed by `+`:

```
$ generate-license GPL-3.0+
warning: 'GPL-3.0+' is a deprecated SPDX identifier, using 'GPL-3.0-or-later'
created: LICENSE
```

### Copyright line

Seven licenses have a copyright line to fill in. The other texts are written
exactly as published.

| Licenses | `--year 2026 --holder "Ada Lovelace"` | `--no-year --holder "Ada Lovelace"` |
|---|---|---|
| `MIT`, `ISC`, `0BSD` | `Copyright (c) 2026 Ada Lovelace` | `Copyright (c) Ada Lovelace` |
| `MIT-0` | `Copyright 2026 Ada Lovelace` | `Copyright Ada Lovelace` |
| `BSD-2-Clause`, `BSD-3-Clause` | `Copyright (c) 2026, Ada Lovelace` | `Copyright (c) Ada Lovelace` |
| `Zlib` | `(C) 2026 Ada Lovelace` | `(C) Ada Lovelace` |

- **Year:** the current year by default. `--year` takes any text, for example
  `--year 2019-2026`. `--no-year` leaves the year out.
- **Holder:** `--holder`, or else the output of `git config --get user.name`.
  If neither gives a name, nothing is written.

The year and the holder are only looked up when a requested license has a
copyright line: `generate-license Apache-2.0` works without git.

### Output streams and exit codes

Only `--list`, `--help`, `--version` and `-o -` write to standard output.
Everything else (`created:`, `unchanged:`, `overwritten:`, `holder:`,
`warning:`, `note:`, `error:`) goes to standard error.

| Exit code | Meaning |
|---|---|
| 0 | Files written or already up to date |
| 1 | Refusal or failure: unknown license, existing file, no holder, I/O error |
| 2 | Usage error |

## Supported licenses

`generate-license --list` prints this table.

| Identifier | Name |
|---|---|
| `0BSD` | BSD Zero Clause License |
| `AGPL-3.0-only` | GNU Affero General Public License v3.0 only |
| `AGPL-3.0-or-later` | GNU Affero General Public License v3.0 or later |
| `Apache-2.0` | Apache License 2.0 |
| `BSD-2-Clause` | BSD 2-Clause "Simplified" License |
| `BSD-3-Clause` | BSD 3-Clause "New" or "Revised" License |
| `BSL-1.0` | Boost Software License 1.0 |
| `CC-BY-4.0` | Creative Commons Attribution 4.0 International |
| `CC-BY-NC-4.0` | Creative Commons Attribution Non Commercial 4.0 International |
| `CC-BY-NC-ND-4.0` | Creative Commons Attribution Non Commercial No Derivatives 4.0 International |
| `CC-BY-NC-SA-4.0` | Creative Commons Attribution Non Commercial Share Alike 4.0 International |
| `CC-BY-ND-4.0` | Creative Commons Attribution No Derivatives 4.0 International |
| `CC-BY-SA-4.0` | Creative Commons Attribution Share Alike 4.0 International |
| `CC0-1.0` | Creative Commons Zero v1.0 Universal |
| `EPL-2.0` | Eclipse Public License 2.0 |
| `EUPL-1.2` | European Union Public License 1.2 |
| `GPL-2.0-only` | GNU General Public License v2.0 only |
| `GPL-2.0-or-later` | GNU General Public License v2.0 or later |
| `GPL-3.0-only` | GNU General Public License v3.0 only |
| `GPL-3.0-or-later` | GNU General Public License v3.0 or later |
| `ISC` | ISC License |
| `LGPL-2.1-only` | GNU Lesser General Public License v2.1 only |
| `LGPL-2.1-or-later` | GNU Lesser General Public License v2.1 or later |
| `LGPL-3.0-only` | GNU Lesser General Public License v3.0 only |
| `LGPL-3.0-or-later` | GNU Lesser General Public License v3.0 or later |
| `MIT` | MIT License |
| `MIT-0` | MIT No Attribution |
| `MPL-2.0` | Mozilla Public License 2.0 |
| `Unlicense` | The Unlicense |
| `Zlib` | zlib License |

The `-only` and `-or-later` variants of a license share the same text.

## File names and existing files

Files are written to the current directory.

- A single file is named `LICENSE`.
- Several files are each named `LICENSE-<identifier>`, except `Apache-2.0`,
  which is named `LICENSE-APACHE`.
- LGPL-3.0 is a set of permissions added to GPL-3.0, so it is always written
  as `COPYING.LESSER` together with the GPL-3.0 text as `COPYING`.

| Command | Files |
|---|---|
| `generate-license MIT` | `LICENSE` |
| `generate-license MIT OR Apache-2.0` | `LICENSE-MIT`, `LICENSE-APACHE` |
| `generate-license MIT AND CC-BY-NC-4.0` | `LICENSE-MIT`, `LICENSE-CC-BY-NC-4.0` |
| `generate-license GPL-3.0-or-later` | `LICENSE` |
| `generate-license GPL-3.0-only OR GPL-3.0-or-later` | `LICENSE` |
| `generate-license LGPL-3.0-only` | `COPYING.LESSER`, `COPYING` |
| `generate-license LGPL-3.0-or-later OR MIT` | `COPYING.LESSER`, `COPYING`, `LICENSE-MIT` |

`-o <path>` chooses the path when exactly one file is written. Its parent
directory must exist. `-o -` prints the text to standard output and touches
nothing on disk.

Everything is checked before anything is written: if one file is refused,
none is written.

| Situation | Result |
|---|---|
| The file does not exist | `created: LICENSE` |
| The file exists with the same content | `unchanged: LICENSE` |
| The file exists with another content | `error: LICENSE already exists and differs (use --force to overwrite)` |
| Another license file is in the directory | `error: other license files found: COPYING, LICENSE.md (remove them, use -o, or pass --force)` |
| `--force` | `overwritten: LICENSE`; other license files are ignored and nothing is ever deleted |

A license file is a file named `LICENSE`, `LICENCE`, `UNLICENSE` or
`COPYING`, in any case, alone or followed by an extension or a suffix such as
`LICENSE.md` or `LICENSE-MIT`. Other license files are not checked when `-o`
is given.

## Verifying downloads

Release archives carry GitHub artifact attestations:

```
gh attestation verify <archive> --repo guysenpai/generate-license
```

## Limitations

- The license set is a selection, not the full SPDX list.
- License exceptions (`WITH`) are not supported.
- The SPDX grammar is not validated: `MIT OR` is accepted and writes MIT.
- The default year is the current year in UTC.
- GitHub does not identify the Creative Commons ND and NC licenses
  (`CC-BY-ND-4.0`, `CC-BY-NC-4.0`, `CC-BY-NC-SA-4.0`, `CC-BY-NC-ND-4.0`).
- The PowerShell installer does not verify checksums, and the shell installer
  only does so when `sha256sum` is available.

## License texts

The texts come from [choosealicense.com](https://choosealicense.com)
([github/choosealicense.com](https://github.com/github/choosealicense.com))
and, for the Creative Commons ND and NC licenses, from
[creativecommons.org](https://creativecommons.org). They are embedded as
published, apart from the normalization described in `licenses/SOURCE`, which
also records the exact revision. `licenses/SHA256SUMS` holds their checksums.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT License](LICENSE-MIT), at your option (`MIT OR Apache-2.0`).
