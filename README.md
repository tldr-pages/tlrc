# tlrc
[![CI](https://github.com/acuteenvy/tlrc/actions/workflows/ci.yml/badge.svg)](https://github.com/acuteenvy/tlrc/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/acuteenvy/tlrc?display_name=tag&color=violet)][latest-release]
[![license](https://img.shields.io/github/license/acuteenvy/tlrc?color=blueviolet)](/LICENSE)
[![downloads](https://img.shields.io/github/downloads/acuteenvy/tlrc/total?color=brightgreen)][latest-release]

A [tldr](https://tldr.sh) client written in Rust.

![screenshot](https://user-images.githubusercontent.com/126529524/234939306-d3da4f33-a2b4-472f-abb7-aab7e4ee84be.png)

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
```bash
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
# Specify a list of desired page languages. If it is empty, all languages are downloaded.
# English is implied and will always be downloaded.
# You can see a list of language codes here: https://github.com/tldr-pages/tldr
# Example: ["de", "pl"]
languages = []

[output]
# Show the command name in the page.
show_title = true
# Strip newlines from output.
compact = false
# Print pages in raw markdown.
raw_markdown = false

# Style for the title of the page (command name).
[style.title]
# Fixed colors:       "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "default"
# 256color ANSI code: { color256 = 50 }
# RGB:                { rgb = [0, 255, 255] }
color = "magenta"
bold = true
underline = false
italic = false

# Style for the description of the page.
[style.description]
color = "magenta"
bold = false
underline = false
italic = false

# Style for the description of examples.
[style.bullet]
color = "green"
bold = false
underline = false
italic = false

# Style for command examples.
[style.example]
color = "cyan"
bold = false
underline = false
italic = false

# Style for URLs inside the description.
[style.url]
color = "red"
bold = false
underline = false
italic = true

# Style for text surrounded by backticks (`).
[style.inline_code]
color = "yellow"
bold = false
underline = false
italic = true

# Style for placeholders inside command examples.
[style.placeholder]
color = "red"
bold = false
underline = false
italic = true
```

[latest-release]: https://github.com/acuteenvy/tlrc/releases/latest
