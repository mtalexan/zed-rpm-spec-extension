# RPM Spec Language Support for Zed

Language support for [RPM `.spec` files](https://rpm-software-management.github.io/rpm/manual/spec.html) in the [Zed editor](https://zed.dev), including syntax highlighting, completions, hover, and more, as well as the injected syntax higlighting rpmbash in scriptlets and sections.

## Features

- **Syntax highlighting** via the [tree-sitter-rpmspec](https://gitlab.com/cryptomilk/tree-sitter-rpmspec) grammar, including rpmbash injection for scriptlet bodies
- **Completions** — architecture value completions after `%ifarch` and `%ifnarch` directives (e.g. `x86_64`, `aarch64`, `%{arm}`)
- **Hover** — shows the defined value of `%global`/`%define` variables on hover; shows descriptions for preamble tags (`Name`, `Version`, `BuildRequires`, etc.)
- **Go to definition** — jump to the `%global` or `%define` declaration for a macro variable
- **Find references** — locate all `%{varName}` usages and `%undefine` calls for a variable
- **Snippets** — templates for full spec files, sub-packages, conditionals, and individual sections
- **Bracket matching** — for `%{…}`, `%(…)`, `${…}`, `[…]`, `(…)`, and quotes
- **RPM Bash syntax highlighting** for scriptlet sections (e.g. `%pre`, `%post`, `%install`, `%check`) via the `rpmbash` tree-sitter grammar, which understands RPM macros as well.

## Installation

Search for **RPM Spec** in the Zed Extensions panel (`zed: extensions`).

## Grammar

Syntax highlighting and language injection use the [`tree-sitter-rpmspec`](https://gitlab.com/cryptomilk/tree-sitter-rpmspec) grammar. This grammar provides:

- Full parsing of RPM spec file syntax
- `rpmbash` — an extended bash grammar for scriptlet sections that understands RPM macros

## Language Server

The `rpm-spec-lsp` language server provides completions, hover, definition, and reference support. It is compiled as a standalone Rust binary.

On first use, Zed will attempt to find `rpm-spec-lsp` on your `PATH`. If it is not found, the extension will download a pre-built binary for your platform from the GitHub Releases page.

Supported platforms for automatic download:

- Linux x86\_64
- macOS x86\_64
- macOS aarch64 (Apple Silicon)

To build and install the server manually:

```sh
cd rpm-spec-lsp
cargo build --release
cp target/release/rpm-spec-lsp ~/.local/bin/
```

## Development

To test the extension locally, install it as a dev extension in Zed:

1. Open Zed
2. Open the Command Palette and run `zed: install dev extension`
3. Select the directory containing this repository

> **WARNING:** If you made changes to the `rpm-spec-lsp` server, you will need to manually build that and place it somewhere in your `PATH` first (e.g. `~/.local/bin`). You can remove this binary after the changes have been published and become available as a GitHub Release for automatic download.

> **WARNING** Zed clones the grammar repositories via HTTPS into `grammars/` when it (re)builds the development extension. If the repos already exist in that folder, it verifies the remote URL is a correct _HTTPS_ URL. If you have git config that rewrites your URLs to SSH (e.g. `url."git@github.com:".insteadOf = "https://github.com/"`) Zed will fail to match this and error out. To work around this, either manually remove the `grammars/` folder before rebuilding so Zed clones fresh copies, or add a local git config override to remove the URL rewrite for this repository.

## Releasing

Pushing a tag of the form `v*` triggers the GitHub Actions workflow that builds `rpm-spec-lsp` for all supported platforms and attaches the binaries to a GitHub Release.

## Licensing

This extension is licensed under the [MIT License](./LICENSE).

### Licensing Considerations

The [VSCode RPM Spec Extension by rv-smartporting](https://github.com/rv-smartporting/rpm-spec-extension), which served as a major reference for this project, is licensed under the Mulan PSL v2 license, which is not directly compatible with the MIT License of this project. No part of that code was copied, but it was used to inform this implementation significantly. 

The [Official RPM `.spec` Specification](https://rpm-software-management.github.io/rpm/manual/spec.html) doesn't have clearly stated licensing for the documentation itself, though the RPM tools are licensed under GPLv2. To avoid possible licensing conflicts, the description and help text used in this extension is **not** taken directly from the official documentation, but is reworded as an original work for this extension.


## Acknowledgements

- [tree-sitter-rpmspec](https://gitlab.com/cryptomilk/tree-sitter-rpmspec) rpmspec and rpmbash grammar by cryptomilk
- [VSCode RPM Spec Extension](https://github.com/rv-smartporting/rpm-spec-extension) for guidance, especially on the LSP server
- [RPM `.spec` Specification](https://rpm-software-management.github.io/rpm/manual/spec.html) for official documentation on the spec file format and semantics
- [The first Zed RPM Spec extension](https://github.com/mtorromeo/zed-rpmspec-language) which required manually installing a custom-build language server before the extension could function.
  - The limitation on needing to manually install the [rpm-spec-language-server](https://github.com/dcermak/rpm-spec-language-server), which isn't packaged anywhere but AUR and isn't expecially well setup for isolated deployment was a driving factor in the development of this alternative extension.