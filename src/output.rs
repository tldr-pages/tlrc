use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

use yansi::Color::Green;
use yansi::{Paint, Style};

use crate::config::Config;
use crate::error::{Error, ErrorKind, Result};
use crate::util::{warnln, PagePathExt};

const TITLE: &str = "# ";
const DESC: &str = "> ";
const BULLET: &str = "- ";
const EXAMPLE: char = '`';

struct RenderStyles {
    title: Style,
    desc: Style,
    bullet: Style,
    example: Style,
    url: Style,
    inline_code: Style,
    placeholder: Style,
}

pub struct PageRenderer<'a> {
    /// Path to the page.
    path: &'a Path,
    /// A BufReader containing the page.
    reader: BufReader<File>,
    /// A buffered handle to standard output.
    stdout: BufWriter<io::StdoutLock<'static>>,
    /// The line of the page that is currently being worked with.
    current_line: String,
    /// The line number of the current line.
    lnum: usize,
    /// Style configuration.
    style: RenderStyles,
    /// Other options.
    cfg: &'a Config,
}

impl<'a> PageRenderer<'a> {
    fn hl_code(&self, s: &str, style_normal: Style) -> String {
        let split: Vec<&str> = s.split('`').collect();
        // Highlight beginning not found.
        if split.len() == 1 {
            return style_normal.paint(s).to_string();
        }

        let mut buf = String::new();

        for (i, part) in split.into_iter().enumerate() {
            // Only odd indexes contain the part to be highlighted.
            // "aa `bb` cc `dd` ee"
            // 0: "aa "
            // 1: "bb"      (highlighted)
            // 2: " cc "
            // 3: "dd"      (highlighted)
            // 4: " ee"
            if i % 2 == 0 {
                buf += &style_normal.paint(part).to_string();
            } else {
                buf += &self.style.inline_code.paint(part).to_string();
            }
        }

        buf
    }

    fn hl_url(&self, s: &str, style_normal: Style) -> String {
        let split: Vec<&str> = s.split("<http").collect();
        // Highlight beginning not found.
        if split.len() == 1 {
            return style_normal.paint(s).to_string();
        }

        let mut buf = String::new();

        for part in split {
            if part.contains('>') {
                // The first part of the second split contains the part to be highlighted.
                //
                // "More information: <https://example.com>."
                // 0: "More information: " => does not match
                // 1: "s://example.com>."  => 0: "s://example.com" (highlighted)
                //                            1: ">."
                let part_split = part.split_once('>').unwrap();

                // "<http" is used to detect URLs. It must be added back.
                let hl = format!("http{}", part_split.0);
                buf += &self.style.url.paint(hl).to_string();
                buf += &style_normal.paint(part_split.1).to_string();
            } else {
                // Highlight ending not found.
                buf += &style_normal.paint(part).to_string();
            }
        }

        buf
    }

    fn hl_placeholder(&self, s: &str, style_normal: Style) -> String {
        let split: Vec<&str> = s.split("{{").collect();
        // Highlight beginning not found.
        if split.len() == 1 {
            return style_normal.paint(s).to_string();
        }

        let mut buf = String::new();

        for part in split {
            if part.contains("}}") {
                // The first part of the second split contains the part to be highlighted.
                //
                // "aa bb {{cc}} {{dd}} ee"
                // 0: "aa bb "   => does not match
                // 1: "cc}} "    => 0: "cc"    (highlighted)
                //                  1: "}}"
                // 2: "dd}} ee"  => 0: "dd"    (highlighted)
                //                  1: "}} ee"

                // This is required for special cases with three closing curly braces ("}}}").
                // The first brace is inside the placeholder, and the last two mark the end of it.
                let idx = part.rmatch_indices("}}").last().unwrap().0;
                let part_split = part.split_at(idx);

                buf += &self.style.placeholder.paint(part_split.0).to_string();
                buf += &style_normal.paint(&part_split.1[2..]).to_string();
            } else {
                // Highlight ending not found.
                buf += &style_normal.paint(part).to_string();
            }
        }

        buf
    }

    /// Print or render the page according to the provided config.
    pub fn print(path: &'a Path, cfg: &'a Config) -> Result<()> {
        let mut page = File::open(path)
            .map_err(|e| Error::new(format!("'{}': {e}", path.display())).kind(ErrorKind::Io))?;

        if cfg.output.raw_markdown {
            io::copy(&mut page, &mut io::stdout()).map_err(|e| {
                Error::new(format!("'{}': {e}", path.display())).kind(ErrorKind::Io)
            })?;
            return Ok(());
        }

        Self {
            path,
            reader: BufReader::new(page),
            stdout: BufWriter::new(io::stdout().lock()),
            current_line: String::new(),
            lnum: 0,
            style: RenderStyles {
                title: cfg.style.title.into(),
                desc: cfg.style.description.into(),
                bullet: cfg.style.bullet.into(),
                example: cfg.style.example.into(),
                url: cfg.style.url.into(),
                inline_code: cfg.style.inline_code.into(),
                placeholder: cfg.style.placeholder.into(),
            },
            cfg,
        }
        .render()
    }

    /// Print the first page that was found and warnings for every other page.
    pub fn print_cache_result(paths: &'a [PathBuf], cfg: &'a Config) -> Result<()> {
        if !crate::QUIET.load(Relaxed) && paths.len() != 1 {
            let mut stderr = io::stderr().lock();
            let other_pages = &paths[1..];
            let width = other_pages
                .iter()
                .map(|x| x.page_platform().unwrap().len())
                .max()
                .unwrap();

            warnln!("{} page(s) found for other platforms:", other_pages.len());

            for (i, path) in other_pages.iter().enumerate() {
                // The path always ends with the page file, and its parent is always the
                // platform directory. This is safe to unwrap.
                let name = path.page_name().unwrap();
                let platform = path.page_platform().unwrap();

                writeln!(
                    stderr,
                    "{} {platform:<width$} (tldr --platform {platform} {name})",
                    Paint::new(format!("{}.", i + 1)).fg(Green).bold(),
                )?;
            }
        }

        // This is safe to unwrap - errors would have already been catched in run().
        let first = paths.first().unwrap();
        Self::print(first, cfg)
    }

    /// Load the next line into the line buffer.
    fn next_line(&mut self) -> Result<usize> {
        self.current_line.clear();
        self.lnum += 1;
        let n = self
            .reader
            .read_line(&mut self.current_line)
            .map_err(|e| Error::new(format!("'{}': {e}", self.path.display())))?;
        self.current_line
            .truncate(self.current_line.trim_end().len());
        Ok(n)
    }

    /// Write the current line to the page buffer as a title.
    fn add_title(&mut self) -> Result<()> {
        if !self.cfg.output.show_title {
            return Ok(());
        }
        self.add_newline()?;

        let line = self.current_line.strip_prefix(TITLE).unwrap();
        let title = if self.cfg.output.platform_title {
            if let Some(platform) = self.path.page_platform() {
                Cow::Owned(format!("{platform}/{line}"))
            } else {
                Cow::Borrowed(line)
            }
        } else {
            Cow::Borrowed(line)
        };

        let title = self.style.title.paint(title);
        let indent = " ".repeat(self.cfg.indent.title);
        writeln!(self.stdout, "{indent}{title}")?;

        Ok(())
    }

    /// Write the current line to the page buffer as a description.
    fn add_desc(&mut self) -> Result<()> {
        let desc = self.hl_code(
            &self.hl_url(
                self.current_line.strip_prefix(DESC).unwrap(),
                self.style.desc,
            ),
            self.style.desc,
        );
        let indent = " ".repeat(self.cfg.indent.description);
        writeln!(self.stdout, "{indent}{desc}")?;

        Ok(())
    }

    /// Write the current line to the page buffer as a bullet point.
    fn add_bullet(&mut self) -> Result<()> {
        let line = if self.cfg.output.show_hyphens {
            self.current_line
                .replace_range(..2, &self.cfg.output.example_prefix);
            &self.current_line
        } else {
            self.current_line.strip_prefix(BULLET).unwrap()
        };

        let bullet = self.hl_code(&self.hl_url(line, self.style.bullet), self.style.bullet);
        let indent = " ".repeat(self.cfg.indent.bullet);
        writeln!(self.stdout, "{indent}{bullet}")?;

        Ok(())
    }

    /// Write the current line to the page buffer as an example.
    fn add_example(&mut self) -> Result<()> {
        // Add spaces around escaped curly braces in order not to
        // interpret them as a placeholder (e.g. in "\{\{{{ }}\}\}").
        self.current_line = self
            .current_line
            .replace("\\{\\{", " \\{\\{ ")
            .replace("\\}\\}", " \\}\\} ");

        let line = self
            .current_line
            .strip_prefix(EXAMPLE)
            .unwrap()
            .strip_suffix('`')
            .ok_or_else(|| {
                Error::parse_page(self.path, self.lnum, &self.current_line)
                    .describe("\nEvery line with an example must end with a backtick '`'.")
            })?;

        let example = self
            .hl_placeholder(line, self.style.example)
            // Remove the extra spaces and backslashes.
            .replace(" \\{\\{ ", "{{")
            .replace(" \\}\\} ", "}}");

        let indent = " ".repeat(self.cfg.indent.example);
        writeln!(self.stdout, "{indent}{example}")?;

        Ok(())
    }

    /// Write a newline to the page buffer if compact mode is not turned on.
    fn add_newline(&mut self) -> Result<()> {
        if !self.cfg.output.compact {
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
            } else if self.current_line.chars().all(char::is_whitespace) {
                self.add_newline()?;
            } else {
                return Err(
                    Error::parse_page(self.path, self.lnum, &self.current_line).describe(
                        "\nEvery non-empty line must begin with either '# ', '> ', '- ' or '`'.",
                    ),
                );
            }
        }

        self.add_newline()?;
        Ok(self.stdout.flush()?)
    }
}
