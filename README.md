# tlrc
[![CI](https://github.com/acuteenvy/tlrc/actions/workflows/ci.yml/badge.svg)](https://github.com/acuteenvy/tlrc/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/acuteenvy/tlrc?display_name=tag&color=violet)][latest-release]
[![license](https://img.shields.io/github/license/acuteenvy/tlrc?color=blueviolet)](/LICENSE)
[![downloads](https://img.shields.io/github/downloads/acuteenvy/tlrc/total?color=brightgreen)][latest-release]

A [tldr](https://tldr.sh) client written in Rust.

![screenshot](https://user-images.githubusercontent.com/126529524/232170100-86a85f13-f9cd-404c-869b-d48ced01557a.png)


## Installation
[![Packaging status](https://repology.org/badge/vertical-allrepos/tlrc.svg)](https://repology.org/project/tlrc/versions)

### From GitHub Releases
You can find prebuilt binaries [here][latest-release].


## Usage
See `man tldr` or the [online manpage](https://acuteenvy.github.io/tlrc). For a brief description, you can also run:
```
tldr --help
```

## Configuration
Tlrc can be customized with a [TOML](https://toml.io) configuration file. To get the default path for your system, run:
```
tldr --config-path
```
To generate a default config file, run:
```
tldr --gen-config
```
or copy the below example.

### Configuration options
```toml
[cache]
# Override the cache directory.
dir = "/home/v/.cache/tlrc"
# Automatically update the cache when it is if it is older than max_age hours.
auto_update = true
max_age = 336
# A list of languages to download. If it is empty, all languages are downloaded.
# You can see a list of language codes here: https://github.com/tldr-pages/tldr
# Example: ["en", "de", "pl"]
languages = []

[output]
# Show the command name in the page.
show_title = true
# Strip newlines from output.
compact = false
# Print pages in raw markdown.
raw_markdown = false

[style.title]
# Fixed colors:       "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "default"
# 256color ANSI code: { color256 = 50 }
# RGB:                { rgb = [0, 255, 255] }
color = "magenta"
bold = true
underline = false
italic = false

[style.description]
color = "magenta"
bold = false
underline = false
italic = false

[style.bullet]
color = "green"
bold = false
underline = false
italic = false

[style.example]
color = "cyan"
bold = false
underline = false
italic = false
```

[latest-release]: https://github.com/acuteenvy/tlrc/releases/latest
