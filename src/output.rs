use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use yansi::Style;

use crate::config::{IndentConfig, OutputConfig, StyleConfig};
use crate::error::{Error, Result};

const TITLE: &str = "# ";
const DESC: &str = "> ";
const BULLET: &str = "- ";
const EXAMPLE: char = '`';

/// Highlight a substring between `from` and `to` inside `s`.
fn highlight_between(
    from: &str,
    to: &str,
    s: &str,
    style_normal: &Style,
    style_hl: &Style,
) -> String {
    let mut result = String::new();

    for (i, spl) in s.split(from).enumerate() {
        if from == to {
            // Only odd indexes contain the part to be highlighted
            if i % 2 == 0 {
                result.push_str(&style_normal.paint(spl).to_string());
            } else {
                result.push_str(&style_hl.paint(spl).to_string());
            }
        } else if spl.contains(to) {
            let mut spl2 = spl.split(to);

            result.push_str(&style_hl.paint(spl2.next().unwrap()).to_string());
            result.push_str(&style_normal.paint(spl2.next().unwrap()).to_string());
        } else {
            result.push_str(&style_normal.paint(spl).to_string());
        }
    }

    result
}

/// Read and print the given page to stdout.
pub fn print_page(
    page_path: &Path,
    outputcfg: &OutputConfig,
    indentcfg: &IndentConfig,
    stylecfg: StyleConfig,
) -> Result<()> {
    let mut reader = BufReader::new(File::open(page_path)?);
    let mut stdout = BufWriter::new(io::stdout().lock());

    if outputcfg.raw_markdown {
        io::copy(&mut reader, &mut stdout)?;
        stdout.flush()?;
        return Ok(());
    }

    let title: Style = stylecfg.title.into();
    let desc: Style = stylecfg.description.into();
    let bullet: Style = stylecfg.bullet.into();
    let example: Style = stylecfg.example.into();
    let url: Style = stylecfg.url.into();
    let inline_code: Style = stylecfg.inline_code.into();
    let placeholder: Style = stylecfg.placeholder.into();

    for (i, line) in reader.lines().enumerate() {
        let line = line?;

        if line.starts_with(TITLE) {
            if !outputcfg.show_title {
                continue;
            }
            if !outputcfg.compact {
                writeln!(stdout)?;
            }
            writeln!(
                stdout,
                "{}{}",
                " ".repeat(indentcfg.title),
                title.paint(&line.strip_prefix(TITLE).unwrap())
            )?;
        } else if line.starts_with(DESC) {
            writeln!(
                stdout,
                "{}{}",
                " ".repeat(indentcfg.description),
                highlight_between(
                    "`",
                    "`",
                    &highlight_between("<", ">", line.strip_prefix(DESC).unwrap(), &desc, &url),
                    &desc,
                    &inline_code
                )
            )?;
        } else if line.starts_with(BULLET) {
            let line = if outputcfg.show_hyphens {
                line.as_str()
            } else {
                line.strip_prefix(BULLET).unwrap()
            };
            writeln!(
                stdout,
                "{}{}",
                " ".repeat(indentcfg.bullet),
                highlight_between("`", "`", line, &bullet, &inline_code)
            )?;
        } else if line.starts_with(EXAMPLE) {
            writeln!(
                stdout,
                "{}{}",
                " ".repeat(indentcfg.example),
                highlight_between(
                    "{{",
                    "}}",
                    line.strip_prefix(EXAMPLE)
                        .unwrap()
                        .strip_suffix(EXAMPLE)
                        .ok_or_else(|| Error::parse_page(page_path, i, &line)
                            .describe("\n\nunclosed backtick '`'"))?,
                    &example,
                    &placeholder,
                )
            )?;
        } else if line.is_empty() {
            if !outputcfg.compact {
                writeln!(stdout)?;
            }
        } else {
            return Err(Error::parse_page(page_path, i, &line)
                .describe("\n\nEvery line must begin with either '#', '> ', '- ' or '`'"));
        }
    }

    if !outputcfg.compact {
        writeln!(stdout)?;
    }

    stdout.flush()?;

    Ok(())
}
