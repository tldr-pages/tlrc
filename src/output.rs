use yansi::Style;

use crate::config::{OutputConfig, StyleConfig};


const TITLE: &str = "# ";
const DESC: &str = "> ";
const BULLET: &str = "- ";
const EXAMPLE: char = '`';

/// Print the given page to stdout.
pub fn print_page(page_string: &str, outputcfg: &OutputConfig, stylecfg: StyleConfig) {
    if outputcfg.raw_markdown {
        print!("{page_string}");
        return;
    }

    let title: Style = stylecfg.title.into();
    let desc: Style = stylecfg.description.into();
    let bullet: Style = stylecfg.bullet.into();
    let example: Style = stylecfg.example.into();

    for line in page_string.lines() {
        if outputcfg.show_title && line.starts_with(TITLE) {
            if !outputcfg.compact {
                println!();
            }
            println!("  {}", title.paint(&line.strip_prefix(TITLE).unwrap()));
        }
        else if line.starts_with(DESC) {
            println!("  {}", desc.paint(&line.strip_prefix(DESC).unwrap()));
        }
        else if line.starts_with(BULLET) {
            println!("  {}", bullet.paint(&line.strip_prefix(BULLET).unwrap()));
        }
        else if line.starts_with(EXAMPLE) {
            println!("      {}", example.paint(&line.strip_prefix(EXAMPLE).unwrap().strip_suffix(EXAMPLE).unwrap()));
        }
        else if !outputcfg.compact && line.is_empty() {
            println!();
        }
    }

    if !outputcfg.compact {
        println!();
    }
}
