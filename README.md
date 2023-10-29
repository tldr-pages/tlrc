<div align="center">

# tlrc

A [tldr](https://tldr.sh) client written in Rust.

[![CI](https://img.shields.io/github/actions/workflow/status/tldr-pages/tlrc/ci.yml?label=CI&logo=github&labelColor=363a4f&logoColor=d9e0ee)](https://github.com/tldr-pages/tlrc/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/tldr-pages/tlrc?&logo=github&color=cba6f7&logoColor=d9e0ee&labelColor=363a4f)][latest-release]
[![crates.io](https://img.shields.io/crates/v/tlrc?&logo=rust&color=cba6f7&logoColor=d9e0ee&labelColor=363a4f)][crate]
[![license](https://img.shields.io/github/license/tldr-pages/tlrc?color=b4befe&labelColor=363a4f)](/LICENSE)
<br>
[![github downloads](https://img.shields.io/github/downloads/tldr-pages/tlrc/total?logo=github&color=94e2d5&logoColor=d9e0ee&labelColor=363a4f)][latest-release]
[![matrix](https://img.shields.io/matrix/tldr-pages%3Amatrix.org?logo=matrix&color=94e2d5&logoColor=d9e0ee&labelColor=363a4f&label=tldr-pages%20matrix)](https://matrix.to/#/#tldr-pages:matrix.org)

![screenshot](https://user-images.githubusercontent.com/126529524/234939306-d3da4f33-a2b4-472f-abb7-aab7e4ee84be.png)

</div>

## Installation

<a href="https://repology.org/project/tlrc/versions">
    <img src="https://repology.org/badge/vertical-allrepos/tlrc.svg" alt="Packaging status" align="right">
</a>

### Arch Linux

Install [tlrc](https://aur.archlinux.org/packages/tlrc) (from source) or [tlrc-bin](https://aur.archlinux.org/packages/tlrc-bin) (prebuilt) from the AUR.

### Windows

Install [tlrc](https://github.com/microsoft/winget-pkgs/tree/master/manifests/t/tldr-pages/tlrc) with `winget`:

```shell
winget install tldr-pages.tlrc
```

### NetBSD

Install [tlrc](https://ftp.netbsd.org/pub/NetBSD/NetBSD-current/pkgsrc/net/tlrc/index.html) with `pkgin`:

```shell
pkgin install tlrc
```

### From crates.io

To build tlrc from a source tarball, run:

```shell
cargo install tlrc
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
tldr --gen-config > $(tldr --config-path)
```

or copy the below example.

### Configuration options

```toml
[cache]
# Override the cache directory.
dir = "/path/to/cache"
# Automatically update the cache when it is if it is older than max_age hours.
auto_update = true
max_age = 336
# Specify a list of desired page languages. If it is empty, languages specified in
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
# Strip empty lines from output.
compact = false
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
# Fixed colors:       "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "default"
# 256color ANSI code: { color256 = 50 }
# RGB:                { rgb = [0, 255, 255] }
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
