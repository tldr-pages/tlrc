use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use yansi::Style;

use crate::config::{IndentConfig, OutputConfig, StyleConfig};
use crate::error::{Error, ErrorKind, Result};

const TITLE: &str = "# ";
const DESC: &str = "> ";
const BULLET: &str = "- ";
const EXAMPLE: char = '`';

/// Highlight a substring between `start` and `end` inside `s` and return a new `String`.
fn highlight(start: &str, end: &str, s: &str, style_normal: &Style, style_hl: &Style) -> String {
    let mut buf = String::new();

    for (i, spl) in s.split(start).enumerate() {
        if start == end {
            // Only odd indexes contain the part to be highlighted.
            // "aa `bb` cc `dd` ee"
            // 0: "aa "
            // 1: "bb"      (highlighted)
            // 2: " cc "
            // 3: "dd"      (highlighted)
            // 4: " ee"
            if i % 2 == 0 {
                buf.push_str(&style_normal.paint(spl).to_string());
            } else {
                buf.push_str(&style_hl.paint(spl).to_string());
            }
        } else if spl.contains(end) {
            // The first part of the second split contains the part to be highlighted.
            // "aa bb {{cc}} {{dd}} ee"
            // 0: "aa bb "   => does not match
            // 1: "cc}} "    => 0: "cc"  (highlighted)
            //                  1: " "
            // 2: "dd}} ee"  => 0: "dd"  (highlighted)
            //                  1: " ee"
            let mut spl2 = spl.split(end);

            buf.push_str(&style_hl.paint(spl2.next().unwrap()).to_string());
            buf.push_str(&style_normal.paint(spl2.next().unwrap()).to_string());
        } else {
            // Highlight ending not found.
            buf.push_str(&style_normal.paint(spl).to_string());
        }
    }

    buf
}

pub struct PageRenderer<'a> {
    /// Path to the page.
    path: &'a Path,
    /// A BufReader containing the page.
    reader: BufReader<File>,
    stdout: BufWriter<io::StdoutLock<'static>>,
    /// The line of the page that is currently being worked with.
    current_line: String,
    /// The line number of the current line.
    lnum: usize,

    title: Style,
    desc: Style,
    bullet: Style,
    example: Style,
    url: Style,
    inline_code: Style,
    placeholder: Style,

    outputcfg: &'a OutputConfig,
    indentcfg: &'a IndentConfig,
}

impl<'a> PageRenderer<'a> {
    /// Print or render the page according to the provided config.
    pub fn print(
        path: &'a Path,
        outputcfg: &'a OutputConfig,
        indentcfg: &'a IndentConfig,
        stylecfg: StyleConfig,
    ) -> Result<()> {
        let mut page = File::open(path)
            .map_err(|e| Error::new(format!("'{}': {e}", path.display())).kind(ErrorKind::Io))?;

        if outputcfg.raw_markdown {
            io::copy(&mut page, &mut io::stdout())?;
            return Ok(());
        }

        Self {
            path,
            reader: BufReader::new(page),
            stdout: BufWriter::new(io::stdout().lock()),
            current_line: String::new(),
            lnum: 0,

            title: stylecfg.title.into(),
            desc: stylecfg.description.into(),
            bullet: stylecfg.bullet.into(),
            example: stylecfg.example.into(),
            url: stylecfg.url.into(),
            inline_code: stylecfg.inline_code.into(),
            placeholder: stylecfg.placeholder.into(),

            outputcfg,
            indentcfg,
        }
        .render()
    }

    /// Load the next line into the line buffer.
    fn next_line(&mut self) -> Result<usize> {
        self.current_line.clear();
        self.lnum += 1;
        Ok(self.reader.read_line(&mut self.current_line)?)
    }

    /// Write the current line to the page buffer as a title.
    fn add_title(&mut self) -> Result<()> {
        if !self.outputcfg.show_title {
            return Ok(());
        }
        if !self.outputcfg.compact {
            writeln!(self.stdout)?;
        }

        Ok(write!(
            self.stdout,
            "{}{}",
            " ".repeat(self.indentcfg.title),
            &self
                .title
                .paint(self.current_line.strip_prefix(TITLE).unwrap())
        )?)
    }

    /// Write the current line to the page buffer as a description.
    fn add_desc(&mut self) -> Result<()> {
        Ok(write!(
            self.stdout,
            "{}{}",
            " ".repeat(self.indentcfg.description),
            highlight(
                "`",
                "`",
                &highlight(
                    "<",
                    ">",
                    self.current_line.strip_prefix(DESC).unwrap(),
                    &self.desc,
                    &self.url,
                ),
                &self.desc,
                &self.inline_code,
            )
        )?)
    }

    /// Write the current line to the page buffer as a bullet point.
    fn add_bullet(&mut self) -> Result<()> {
        let line = if self.outputcfg.show_hyphens {
            self.current_line
                .replace_range(..2, &self.outputcfg.example_prefix);
            self.current_line.as_str()
        } else {
            self.current_line.strip_prefix(BULLET).unwrap()
        };

        Ok(write!(
            self.stdout,
            "{}{}",
            " ".repeat(self.indentcfg.bullet),
            highlight("`", "`", line, &self.bullet, &self.inline_code),
        )?)
    }

    /// Write the current line to the page buffer as an example.
    fn add_example(&mut self) -> Result<()> {
        Ok(writeln!(
            self.stdout,
            "{}{}",
            " ".repeat(self.indentcfg.example),
            highlight(
                "{{",
                "}}",
                self.current_line
                    .strip_prefix(EXAMPLE)
                    .unwrap()
                    .strip_suffix("`\n")
                    .ok_or_else(|| {
                        Error::parse_page(self.path, self.lnum, &self.current_line)
                            .describe("\nEvery line with an example must end with a backtick '`'.")
                    })?,
                &self.example,
                &self.placeholder,
            )
        )?)
    }

    /// Write a newline to the page buffer if compact mode is not turned on.
    fn add_newline(&mut self) -> Result<()> {
        if !self.outputcfg.compact {
            writeln!(self.stdout)?;
        }
        Ok(())
    }

    /// Render the page to standard output.
    fn render(&mut self) -> Result<()> {
        while self.next_line()? != 0 {
            if self.current_line.starts_with(TITLE) {
                self.add_title()?;
            } else if self.current_line.starts_with(DESC) {
                self.add_desc()?;
            } else if self.current_line.starts_with(BULLET) {
                self.add_bullet()?;
            } else if self.current_line.starts_with(EXAMPLE) {
                self.add_example()?;
            } else if self.current_line == "\n" {
                self.add_newline()?;
            } else {
                return Err(
                    Error::parse_page(self.path, self.lnum, &self.current_line).describe(
                        "\nEvery non-empty line must begin with either '#', '> ', '- ' or '`'.",
                    ),
                );
            }
        }
        self.add_newline()?;
        Ok(self.stdout.flush()?)
    }
}
