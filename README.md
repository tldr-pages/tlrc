<div align="center">

# tlrc

A [tldr](https://github.com/tldr-pages/tldr) client written in Rust.

[![CI](https://img.shields.io/github/actions/workflow/status/tldr-pages/tlrc/ci.yml?label=CI&logo=github&labelColor=363a4f&logoColor=d9e0ee)](https://github.com/tldr-pages/tlrc/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/tldr-pages/tlrc?&logo=github&color=cba6f7&logoColor=d9e0ee&labelColor=363a4f)][latest-release]
[![crates.io](https://img.shields.io/crates/v/tlrc?&logo=rust&color=cba6f7&logoColor=d9e0ee&labelColor=363a4f)][crate]
[![license](https://img.shields.io/github/license/tldr-pages/tlrc?color=b4befe&labelColor=363a4f)](/LICENSE)
<br>
[![github downloads](https://img.shields.io/github/downloads/tldr-pages/tlrc/total?logo=github&color=94e2d5&logoColor=d9e0ee&labelColor=363a4f)][latest-release]
[![matrix](https://img.shields.io/matrix/tldr-pages%3Amatrix.org?logo=matrix&color=94e2d5&logoColor=d9e0ee&labelColor=363a4f&label=tldr-pages%20matrix)](https://matrix.to/#/#tldr-pages:matrix.org)

![screenshot](https://github.com/tldr-pages/tlrc/assets/126529524/daa76702-f437-4a99-adfb-7830a6f33eb9)

</div>

## Installation

<a href="https://repology.org/project/tlrc/versions">
    <img src="https://repology.org/badge/vertical-allrepos/tlrc.svg?exclude_unsupported=1" alt="Packaging status" align="right">
</a>

### Linux/macOS using Homebrew

Install [tlrc](https://formulae.brew.sh/formula/tlrc) with Homebrew:

```shell
brew install tlrc
```

### Linux/macOS using Nix

Install [tlrc](https://search.nixos.org/packages?channel=unstable&show=tlrc) from nixpkgs.

### Arch Linux

Install [tlrc](https://aur.archlinux.org/packages/tlrc) (from source) or [tlrc-bin](https://aur.archlinux.org/packages/tlrc-bin) (prebuilt) from the AUR.

### openSUSE

Install [tlrc](https://software.opensuse.org/package/tlrc) with Zypper:

```shell
zypper install tlrc
```

### Windows using Winget

Install [tlrc](https://github.com/microsoft/winget-pkgs/tree/master/manifests/t/tldr-pages/tlrc) with Winget:

```shell
winget install tldr-pages.tlrc
```

### Windows using Scoop

Install [tlrc](https://scoop.sh/#/apps?q=tlrc&id=67f36cdb01b1573ed454af11605b7b8efc732dc7) with Scoop:

```shell
scoop install tlrc
```

### macOS using MacPorts

Install [tlrc](https://ports.macports.org/port/tlrc/details) with MacPorts:

```shell
port install tlrc
```

### NetBSD

Install [tlrc](https://ftp.netbsd.org/pub/NetBSD/NetBSD-current/pkgsrc/net/tlrc/index.html) with `pkgin`:

```shell
pkgin install tlrc
```

### From crates.io

To build [tlrc][crate] from a source tarball, run:

```shell
cargo install tlrc --locked
```

> [!NOTE]
> Shell completion files and the man page will not be installed that way.

### From GitHub Releases

You can find prebuilt binaries [here][latest-release].

## Usage

See `man tldr` or the [online manpage](https://tldr.sh/tlrc). For a brief description, you can also run:

```shell
tldr --help
```

## Configuration

Tlrc can be customized with a [TOML](https://toml.io) configuration file. To get the default path for your system, run:

```shell
tldr --config-path
```

To generate a default config file, run:

```shell
tldr --gen-config > "$(tldr --config-path)"
```

or copy the below example.

### Configuration options

```toml
[cache]
# Override the cache directory ('~' will be expanded to your home directory).
dir = "/path/to/cache"
# Override the base URL used for downloading tldr pages.
# The mirror must provide files with the same names as the official tldr pages repository:
# mirror/tldr.sha256sums            must point to the SHA256 checksums of all assets
# mirror/tldr-pages.LANGUAGE.zip    must point to a zip archive that contains platform directories with pages in LANGUAGE
mirror = "https://github.com/tldr-pages/tldr/releases/latest/download"
# Automatically update the cache if it's older than max_age hours.
auto_update = true
max_age = 336 # 336 hours = 2 weeks
# Defers cache automatic update until after displaying the page.
defer_auto_update = false
# Specify a list of desired page languages. If it's empty, languages specified in
# the LANG and LANGUAGE environment variables are downloaded.
# English is implied and will always be downloaded.
# You can see a list of language codes here: https://github.com/tldr-pages/tldr
# Example: ["de", "pl"]
languages = []

[output]
# Show the title in the rendered page.
show_title = true
# Show the platform name ('common', 'linux', etc.) in the title.
platform_title = false
# Prefix descriptions of examples with hyphens.
show_hyphens = false
# Use a custom string instead of a hyphen.
example_prefix = "- "
# Set the max line length. 0 means to use the terminal width.
# If a description is longer than this value, it will be split
# into multiple lines.
line_length = 0
# Strip empty lines from output.
compact = false
# In option placeholders, show the specified option style.
# Example: {{[-s|--long]}}
# short  : -s
# long   : --long
# both   : [-s|--long]
option_style = "long"
# Print pages in raw markdown.
raw_markdown = false

# Number of spaces to put before each line of the page.
[indent]
# Command name.
title = 2
# Command description.
description = 2
# Descriptions of examples.
bullet = 2
# Example command invocations.
example = 4

# Style for the title of the page (command name).
[style.title]
# Fixed colors:       "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "default",
#                     "bright_black", "bright_red", "bright_green", "bright_yellow", "bright_blue",
#                     "bright_magenta", "bright_cyan", "bright_white"
# 256color ANSI code: { color256 = 50 }
# RGB:                { rgb = [0, 255, 255] }
# Hex:                { hex = "#ffffff" }
color = "magenta"
background = "default"
bold = true
underline = false
italic = false
dim = false
strikethrough = false

# Style for the description of the page.
[style.description]
color = "magenta"
background = "default"
bold = false
underline = false
italic = false
dim = false
strikethrough = false

# Style for descriptions of examples.
[style.bullet]
color = "green"
background = "default"
bold = false
underline = false
italic = false
dim = false
strikethrough = false

# Style for command examples.
[style.example]
color = "cyan"
background = "default"
bold = false
underline = false
italic = false
dim = false
strikethrough = false

# Style for URLs inside the description.
[style.url]
color = "red"
background = "default"
bold = false
underline = false
italic = true
dim = false
strikethrough = false

# Style for text surrounded by backticks (`).
[style.inline_code]
color = "yellow"
background = "default"
bold = false
underline = false
italic = true
dim = false
strikethrough = false

# Style for placeholders inside command examples.
[style.placeholder]
color = "red"
background = "default"
bold = false
underline = false
italic = true
dim = false
strikethrough = false
```

[latest-release]: https://github.com/tldr-pages/tlrc/releases/latest
[crate]: https://crates.io/crates/tlrc
