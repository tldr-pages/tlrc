use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Write};
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::sync::atomic::Ordering;
use std::time::Duration;

use yansi::{Color, Paint};
use zip::ZipArchive;

use crate::args::Platform;
use crate::error::{Error, Result};
use crate::util::{infoln, languages_to_langdirs, warnln};

const ARCHIVE: &str = "https://tldr.sh/assets/tldr.zip";

pub struct Cache(PathBuf);

impl Cache {
    pub fn new<P>(dir: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self(dir.into())
    }

    /// Get the default path to the cache.
    pub fn locate() -> PathBuf {
        dirs::cache_dir().unwrap().join(clap::crate_name!())
    }

    /// Return `true` if the cache directory exists.
    pub fn exists(&self) -> bool {
        self.0.join("pages").is_dir()
    }

    /// Download the tldr pages archive.
    fn download() -> Result<Vec<u8>> {
        let mut buf = vec![];

        infoln!("downloading tldr pages from '{ARCHIVE}'...");
        ureq::get(ARCHIVE)
            .call()?
            .into_reader()
            .read_to_end(&mut buf)?;

        Ok(buf)
    }

    /// Extract `dir` from `archive` and update the page counters.
    fn extract_dir(
        &self,
        archive: &mut ZipArchive<Cursor<Vec<u8>>>,
        files: &[String],
        dir: &str,
        n_existing: usize,
        all_downloaded: &mut usize,
        all_new: &mut usize,
    ) -> Result<()> {
        if !crate::QUIET.load(Ordering::Relaxed) {
            write!(
                io::stderr(),
                "{} extracting '{dir}'...",
                Paint::new("info:").fg(Color::Cyan).bold()
            )?;
        }

        let mut n_downloaded = 0;
        for f in files {
            // Skip directory entries, files that are not in a directory (we want only pages)
            // and files that are not in the specified directory.
            if f.ends_with('/') || !f.contains('/') || !f.starts_with(&format!("{dir}/")) {
                continue;
            }

            let path = self.0.join(f);
            fs::create_dir_all(path.parent().unwrap())?;

            let mut page = archive.by_name(f).unwrap();
            let mut file = File::create(&path)?;

            io::copy(&mut page, &mut file)?;
            n_downloaded += 1;
        }

        let n_new = n_downloaded - n_existing;
        *all_downloaded += n_downloaded;
        *all_new += n_new;

        if !crate::QUIET.load(Ordering::Relaxed) {
            writeln!(
                io::stderr(),
                " {} pages , {} new",
                Paint::new(n_downloaded).fg(Color::Green).bold(),
                Paint::new(n_new).fg(Color::Green).bold(),
            )?;
        }

        Ok(())
    }

    /// Delete the old cache and replace it with a fresh copy.
    pub fn update(&self, languages: &[String]) -> Result<()> {
        let mut archive = ZipArchive::new(Cursor::new(Self::download()?))?;
        let files: Vec<String> = archive.file_names().map(String::from).collect();
        let mut all_downloaded = 0;
        let mut all_new = 0;

        // This HashMap stores language directories and the number of pages
        // in them before the update.
        let mut dirs_npages = HashMap::new();
        let lang_dirs = languages_to_langdirs(languages);

        // English pages are always extracted, so we have to check if they are not
        // explicitly specified.
        if !languages.contains(&"en".to_string()) {
            dirs_npages.insert("pages".to_string(), self.list_all_vec("pages")?.len());
        }

        for lang_dir in &lang_dirs {
            dirs_npages.insert(lang_dir.to_string(), self.list_all_vec(lang_dir)?.len());
        }

        self.clean()?;

        // Always extract English pages, even when not specified in the config.
        if !languages.contains(&"en".to_string()) {
            self.extract_dir(
                &mut archive,
                &files,
                "pages",
                dirs_npages["pages"],
                &mut all_downloaded,
                &mut all_new,
            )?;
        }

        for lang_dir in &lang_dirs {
            // Skip invalid languages.
            if !files.contains(&format!("{lang_dir}/")) {
                continue;
            }

            self.extract_dir(
                &mut archive,
                &files,
                lang_dir,
                *dirs_npages.get(lang_dir).unwrap_or(&0),
                &mut all_downloaded,
                &mut all_new,
            )?;
        }

        infoln!(
            "cache update successful (total: {} pages, {} new).",
            Paint::new(all_downloaded).fg(Color::Green).bold(),
            Paint::new(all_new).fg(Color::Green).bold(),
        );

        Ok(())
    }

    /// Delete the cache directory.
    pub fn clean(&self) -> Result<()> {
        if !self.exists() {
            infoln!("cache does not exist, not cleaning.");
            fs::create_dir_all(&self.0)?;
            return Ok(());
        }

        infoln!("cleaning the cache directory...");
        fs::remove_dir_all(&self.0)?;
        fs::create_dir_all(&self.0)?;

        Ok(())
    }

    /// Find a page for the given platform.
    fn find_page_for(
        &self,
        page_file: &str,
        platform_dir: &str,
        language_dirs: &[String],
    ) -> Option<PathBuf> {
        for lang_dir in language_dirs {
            let path = self.0.join(lang_dir).join(platform_dir).join(page_file);

            if path.is_file() {
                return Some(path);
            }
        }
        None
    }

    /// If the page exists, return the path to it.
    pub fn find(&self, page: &str, languages: &[String], platform: &Platform) -> Result<PathBuf> {
        let page_file = format!("{page}.md");
        let language_dirs = languages_to_langdirs(languages);

        if platform != &Platform::Common {
            if let Some(page_path) =
                self.find_page_for(&page_file, &platform.to_string(), &language_dirs)
            {
                return Ok(page_path);
            }
        }

        if let Some(page_path) = self.find_page_for(&page_file, "common", &language_dirs) {
            return Ok(page_path);
        }

        let mut platforms = vec![
            Platform::Linux,
            Platform::Windows,
            Platform::OsX,
            Platform::Android,
            Platform::SunOs,
        ];
        platforms.retain(|item| item != platform);

        for alt_platform in platforms {
            if let Some(page_path) =
                self.find_page_for(&page_file, &alt_platform.to_string(), &language_dirs)
            {
                if platform == &Platform::Common {
                    warnln!(
                        "showing page from platform '{alt_platform}', \
                        because '{page}' does not exist in 'common'"
                    );
                } else {
                    warnln!(
                        "showing page from platform '{alt_platform}', \
                        because '{page}' does not exist in '{platform}' and 'common'"
                    );
                }
                return Ok(page_path);
            }
        }

        Err(Error::new("page not found."))
    }

    /// List all available pages in `lang` for `platform`.
    fn list_dir(&self, platform: &str, lang_dir: &str) -> Result<Vec<OsString>> {
        if let Ok(entries) = fs::read_dir(self.0.join(lang_dir).join(platform)) {
            Ok(entries
                .map(|res| res.map(|e| e.file_name()))
                .collect::<StdResult<Vec<OsString>, io::Error>>()?)
        } else {
            // If the directory does not exist, return an empty Vec instead of an error.
            Ok(vec![])
        }
    }

    fn print_basenames(pages: &mut Vec<OsString>) -> Result<()> {
        pages.sort();
        pages.dedup();

        let mut stdout = io::stdout().lock();
        for page in pages {
            let str = page.to_string_lossy();
            let page = str.strip_suffix(".md").ok_or_else(|| {
                Error::new(format!(
                    "'{str}': every page file should have a '.md' extension",
                ))
            })?;
            writeln!(stdout, "{page}")?;
        }

        Ok(())
    }

    /// List all pages in English for `platform` and common.
    pub fn list_platform(&self, platform: &Platform) -> Result<()> {
        let mut pages = if platform == &Platform::Common {
            self.list_dir(&platform.to_string(), "pages")?
        } else {
            self.list_dir(&platform.to_string(), "pages")?
                .into_iter()
                .chain(self.list_dir("common", "pages")?.into_iter())
                .collect()
        };

        Self::print_basenames(&mut pages)
    }

    /// List all pages in `lang` and return a `Vec`.
    fn list_all_vec(&self, lang_dir: &str) -> Result<Vec<OsString>> {
        Ok(self
            .list_dir("linux", lang_dir)?
            .into_iter()
            .chain(self.list_dir("osx", lang_dir)?)
            .chain(self.list_dir("windows", lang_dir)?)
            .chain(self.list_dir("android", lang_dir)?)
            .chain(self.list_dir("sunos", lang_dir)?)
            .chain(self.list_dir("common", lang_dir)?)
            .collect())
    }

    /// List all pages in English.
    pub fn list_all(&self) -> Result<()> {
        Self::print_basenames(&mut self.list_all_vec("pages")?)
    }

    /// Return `true` if the cache is older than `max_age`.
    pub fn is_stale(&self, max_age: &Duration) -> Result<bool> {
        let since = fs::metadata(self.0.join("pages"))?
            .modified()?
            .elapsed()
            .map_err(|_| {
                Error::new(
                    "the system clock is not functioning correctly.\n\
                    Modification time of the cache is later than the current system time.\n\
                    Please fix your system clock.",
                )
            })?;

        if &since > max_age {
            return Ok(true);
        }

        Ok(false)
    }
}
