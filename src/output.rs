use std::borrow::Cow;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

use terminal_size::terminal_size;
use unicode_width::UnicodeWidthStr;
use yansi::{Paint, Style};

use crate::config::{Config, OptionStyle};
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

/// Type of the line.
/// Does not include types where there's nothing to highlight (i.e. title and empty lines).
#[derive(Clone, Copy, PartialEq)]
enum LineType {
    Desc,
    Bullet,
    Example,
}

pub struct PageRenderer<'a> {
    /// Path to the page.
    path: &'a Path,
    /// A buffered reader containing the page.
    reader: BufReader<File>,
    /// A buffered handle to standard output.
    stdout: BufWriter<io::StdoutLock<'static>>,
    /// The line of the page that is currently being worked with.
    current_line: String,
    /// The line number of the current line.
    lnum: usize,
    /// Max line length.
    max_len: Option<usize>,
    /// Style configuration.
    style: RenderStyles,
    /// Other options.
    cfg: &'a Config,
}

/// Write a `yansi::Painted` to a `String`.
///
/// This is used to append something to a `String` without creating `String`s for every part of a
/// line that's highlighted using a different style.
macro_rules! write_paint {
    ($buf:expr, $what:expr) => {
        // This will never return an error, we're writing to a `String`.
        let _ = write!($buf, "{}", $what);
    };
}

impl<'a> PageRenderer<'a> {
    fn hl_code(&self, s: &str, style_normal: Style) -> String {
        let split: Vec<&str> = s.split('`').collect();
        let mut buf = String::with_capacity(s.len());

        // Highlight beginning not found.
        if split.len() == 1 {
            write_paint!(buf, s.paint(style_normal));
            return buf;
        }

        for (i, part) in split.into_iter().enumerate() {
            // Only odd indexes contain the part to be highlighted.
            // "aa `bb` cc `dd` ee"
            // 0: "aa "
            // 1: "bb"      (highlighted)
            // 2: " cc "
            // 3: "dd"      (highlighted)
            // 4: " ee"
            if i % 2 == 0 {
                write_paint!(buf, part.paint(style_normal));
            } else {
                write_paint!(buf, part.paint(self.style.inline_code));
            }
        }

        buf
    }

    fn hl_url(&self, s: &str, style_normal: Style) -> String {
        let split: Vec<&str> = s.split("<http").collect();
        let mut buf = String::with_capacity(s.len());

        // Highlight beginning not found.
        if split.len() == 1 {
            write_paint!(buf, s.paint(style_normal));
            return buf;
        }

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
                write_paint!(buf, hl.paint(self.style.url));
                write_paint!(buf, part_split.1.paint(style_normal));
            } else {
                // Highlight ending not found.
                write_paint!(buf, part.paint(style_normal));
            }
        }

        buf
    }

    fn hl_placeholder(&self, s: &str, style_normal: Style) -> String {
        let split: Vec<&str> = s.split("{{").collect();
        let mut buf = String::with_capacity(s.len());

        // Highlight beginning not found.
        if split.len() == 1 {
            write_paint!(buf, s.paint(style_normal));
            return buf;
        }

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
                let (inside, outside) = part.split_at(idx);

                // Select the long or short option.
                // Skip if the user wants to display both or if the placeholder doesn't contain
                // option selection (`[-s|--long]`).
                if self.cfg.output.option_style != OptionStyle::Both
                    && inside.starts_with('[')
                    && inside.ends_with(']')
                    && inside.contains('|')
                {
                    let (short, long) = inside.split_once('|').unwrap();
                    // A single option will be displayed, using the normal style (static part).
                    if self.cfg.output.option_style == OptionStyle::Short {
                        // Cut out the leading `[`.
                        write_paint!(buf, &short[1..].paint(style_normal));
                    } else {
                        // Cut out the trailing `]`.
                        write_paint!(buf, &long[..long.len() - 1].paint(style_normal));
                    }
                } else {
                    // Both options will be displayed, or this isn't an option placeholder.
                    // The placeholder style is used in both cases.
                    write_paint!(buf, inside.paint(self.style.placeholder));
                }

                // `outside` begins with "}}". We need to cut that out.
                write_paint!(buf, &outside[2..].paint(style_normal));
            } else {
                // Highlight ending not found.
                write_paint!(buf, part.paint(style_normal));
            }
        }

        buf
    }

    /// Split the line into multiple lines if it's longer than the configured max length.
    fn splitln(
        &self,
        s: &'a str,
        indent: &str,
        prefix_width: usize,
        ltype: LineType,
    ) -> Cow<'a, str> {
        let Some(max_len) = self.max_len else {
            // We don't have the max length. Just print the entire line then.
            return Cow::Borrowed(s);
        };
        let len_indent = indent.len();

        if len_indent + s.width() <= max_len {
            // The line is shorter than the max length. There is nothing to do.
            return Cow::Borrowed(s);
        }

        let words = s.split(' ');
        let len_s = s.len();
        let base_width = len_indent + prefix_width;
        let mut cur_width = base_width;
        //                  current_len + base_width * amount of added newlines
        let mut buf = String::with_capacity(len_s + base_width * (len_s / max_len));

        // If the example prefix is set, we need more whitespace at the beginning of the next line.
        let indent = if prefix_width == 0 {
            Cow::Borrowed(indent)
        } else {
            Cow::Owned(" ".repeat(prefix_width) + indent)
        };

        #[allow(clippy::items_after_statements)]
        enum InsideHl {
            Code,
            Placeholder,
            NotInside,
        }

        // Are we inside something highlighted (i.e. backticks or placeholders)?
        let mut inside_hl = InsideHl::NotInside;

        let style_normal = match ltype {
            LineType::Desc => self.style.desc,
            LineType::Bullet => self.style.bullet,
            LineType::Example => self.style.example,
        };

        for w in words {
            if ltype == LineType::Example && w.contains("{{") {
                inside_hl = InsideHl::Placeholder;
            }
            let w_width = w.width();

            if cur_width + w_width > max_len && cur_width != base_width {
                // If the next word is added, the line will be longer than the configured line
                // length.
                //
                // We need to add a newline + indentation, and reset the current length.
                if yansi::is_enabled() {
                    // Style reset. Without this, whitespace will have a background color (if one
                    // is set).
                    let _ = style_normal.fmt_suffix(&mut buf);
                }
                buf.push('\n');
                buf += &indent;
                if yansi::is_enabled() {
                    // Reenable the style.
                    let _ = match inside_hl {
                        InsideHl::Code => self.style.inline_code.fmt_prefix(&mut buf),
                        InsideHl::Placeholder => self.style.placeholder.fmt_prefix(&mut buf),
                        InsideHl::NotInside => style_normal.fmt_prefix(&mut buf),
                    };
                }
                cur_width = base_width;
            } else if cur_width != base_width {
                // If this isn't the beginning of the line, add a space after the word.
                buf.push(' ');
                cur_width += 1;
            }

            buf += w;
            cur_width += w_width;

            if ltype != LineType::Example && w.chars().filter(|x| *x == '`').count() == 1 {
                inside_hl = match inside_hl {
                    InsideHl::NotInside => InsideHl::Code,
                    InsideHl::Code => InsideHl::NotInside,
                    // If this line isn't an example, there is no way this happens.
                    InsideHl::Placeholder => unreachable!(),
                };
            } else if ltype == LineType::Example && w.contains("}}") {
                inside_hl = InsideHl::NotInside;
            }
        }

        Cow::Owned(buf)
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
            max_len: if cfg.output.line_length == 0 {
                terminal_size().map(|x| x.0 .0 as usize)
            } else {
                Some(cfg.output.line_length)
            },
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
                    format!("{}.", i + 1).green().bold(),
                )?;
            }
        }

        // This is safe to unwrap - errors would have already been catched in run().
        let first = paths.first().unwrap();
        Self::print(first, cfg)
    }

    /// Load the next line into the line buffer.
    fn next_line(&mut self) -> Result<usize> {
        // The `Paint` trait from yansi also has a method named `clear`.
        // This will be resolved in a future release: https://github.com/SergioBenitez/yansi/issues/42
        //self.current_line.clear();
        String::clear(&mut self.current_line);
        self.lnum += 1;
        let n = self
            .reader
            .read_line(&mut self.current_line)
            .map_err(|e| Error::new(format!("'{}': {e}", self.path.display())))?;
        let len = self.current_line.trim_end().len();
        self.current_line.truncate(len);

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

        let title = title.paint(self.style.title);
        let indent = " ".repeat(self.cfg.indent.title);
        writeln!(self.stdout, "{indent}{title}")?;

        Ok(())
    }

    /// Write the current line to the page buffer as a description.
    fn add_desc(&mut self) -> Result<()> {
        let indent = " ".repeat(self.cfg.indent.description);
        let line = self.current_line.strip_prefix(DESC).unwrap();
        let line = self.splitln(line, &indent, 0, LineType::Desc);
        let desc = self.hl_code(&self.hl_url(&line, self.style.desc), self.style.desc);

        writeln!(self.stdout, "{indent}{desc}")?;

        Ok(())
    }

    /// Write the current line to the page buffer as a bullet point.
    fn add_bullet(&mut self) -> Result<()> {
        let indent = " ".repeat(self.cfg.indent.bullet);
        let line = if self.cfg.output.show_hyphens {
            self.current_line
                .replace_range(..2, &self.cfg.output.example_prefix);
            self.splitln(
                &self.current_line,
                &indent,
                self.cfg.output.example_prefix.width(),
                LineType::Bullet,
            )
        } else {
            let l = self.current_line.strip_prefix(BULLET).unwrap();
            self.splitln(l, &indent, 0, LineType::Bullet)
        };

        let bullet = self.hl_code(&self.hl_url(&line, self.style.bullet), self.style.bullet);
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

        let indent = " ".repeat(self.cfg.indent.example);
        let line = self.splitln(
            self.current_line
                .strip_prefix(EXAMPLE)
                .unwrap()
                .strip_suffix('`')
                .ok_or_else(|| {
                    Error::parse_page(self.path, self.lnum, &self.current_line)
                        .describe("\nEvery line with an example must end with a backtick '`'.")
                })?,
            &indent,
            0,
            LineType::Example,
        );

        let example = self
            .hl_placeholder(&line, self.style.example)
            // Remove the extra spaces and backslashes.
            .replace(" \\{\\{ ", "{{")
            .replace(" \\}\\} ", "}}");

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
