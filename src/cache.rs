use std::collections::{BTreeMap, HashMap};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::time::Duration;

use yansi::{Color, Paint};
use zip::ZipArchive;

use crate::args::Platform;
use crate::error::{Error, Result};
use crate::util::{info_end, info_start, infoln, languages_to_langdirs, sha256_hexdigest, warnln};

const BASE_ARCHIVE_URL: &str =
    "https://raw.githubusercontent.com/tldr-pages/tldr-pages.github.io/main/assets";
const CHECKSUMS: &str =
    "https://raw.githubusercontent.com/tldr-pages/tldr-pages.github.io/main/assets/tldr.sha256sums";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), '/', env!("CARGO_PKG_VERSION"));
const ENGLISH_DIR: &str = "pages.en";

type PagesArchive = ZipArchive<Cursor<Vec<u8>>>;

pub struct Cache<'a>(&'a Path);

impl<'a> Cache<'a> {
    pub fn new(dir: &'a Path) -> Self {
        Self(dir)
    }

    /// Get the default path to the cache.
    pub fn locate() -> PathBuf {
        dirs::cache_dir().unwrap().join(env!("CARGO_PKG_NAME"))
    }

    /// Return `true` if the cache directory exists.
    pub fn exists(&self) -> bool {
        self.0.is_dir()
    }

    /// Download the tldr pages archives.
    fn download_and_verify(mut languages: Vec<String>) -> Result<BTreeMap<String, PagesArchive>> {
        let agent = ureq::builder().user_agent(USER_AGENT).build();
        let mut langdir_archive_map = BTreeMap::new();

        infoln!("downloading 'tldr.sha256sums'...");
        let sums = agent.get(CHECKSUMS).call()?.into_string()?;
        let lang_sum_map = Self::parse_sumfile(&sums)?;

        languages.sort_unstable();
        // The user can put duplicates in the config file.
        languages.dedup();
        for lang in &languages {
            let sum = lang_sum_map.get(lang);
            // Skip nonexistent languages.
            if sum.is_none() {
                continue;
            }
            let sum = sum.unwrap();

            infoln!("downloading 'tldr-pages.{lang}.zip'...");

            let mut archive = vec![];
            agent
                .get(&format!("{BASE_ARCHIVE_URL}/tldr-pages.{lang}.zip"))
                .call()?
                .into_reader()
                .read_to_end(&mut archive)?;

            info_start!("validating sha256sums...");
            let actual_sum = sha256_hexdigest(&archive);

            if sum != &actual_sum {
                info_end!(" {}", Paint::new("FAILED").fg(Color::Red).bold());
                return Err(Error::new(format!(
                    "SHA256 sum mismatch!\n\
                    expected : {sum}\n\
                    got      : {actual_sum}"
                )));
            }

            info_end!(" {}", Paint::new("OK").fg(Color::Green).bold());

            langdir_archive_map.insert(
                format!("pages.{lang}"),
                ZipArchive::new(Cursor::new(archive))?,
            );
        }

        Ok(langdir_archive_map)
    }

    fn parse_sumfile(s: &str) -> Result<HashMap<String, String>> {
        let mut map = HashMap::new();

        for l in s.lines() {
            // The file looks like this:
            // sha256sum     path/to/tldr-pages.lang.zip
            // sha256sum     path/to/tldr-pages.lang.zip
            // ...

            let mut spl = l.split_whitespace();
            let sum = spl.next().ok_or_else(Error::parse_sumfile)?;
            let path = spl.next().ok_or_else(Error::parse_sumfile)?;

            // Skip other files, the full archive, and the old English archive
            // (tldr-pages.en.zip is now available).
            if !path.ends_with("zip")
                || path.ends_with("tldr.zip")
                || path.ends_with("tldr-pages.zip")
            {
                continue;
            }

            let lang = path.split('.').nth(1).ok_or_else(Error::parse_sumfile)?;
            map.insert(lang.to_string(), sum.to_string());
        }

        Ok(map)
    }

    /// Extract pages from the language archive and update the page counters.
    fn extract_lang_archive(
        &self,
        lang_dir: &str,
        archive: &mut PagesArchive,
        n_existing: usize,
        all_downloaded: &mut usize,
        all_new: &mut usize,
    ) -> Result<()> {
        info_start!("extracting '{lang_dir}'...");

        let mut n_downloaded = 0;
        let files: Vec<String> = archive.file_names().map(String::from).collect();

        for f in &files {
            // Skip directory entries and files that are not in a directory (we want only pages).
            if f.ends_with('/') || !f.contains('/') {
                continue;
            }

            let path = self.0.join(lang_dir).join(f);
            fs::create_dir_all(path.parent().unwrap())?;

            let mut page = archive.by_name(f).unwrap();
            let mut file = File::create(&path)?;

            io::copy(&mut page, &mut file)?;
            n_downloaded += 1;
        }

        let n_new = n_downloaded - n_existing;
        *all_downloaded += n_downloaded;
        *all_new += n_new;

        info_end!(
            " {} pages, {} new",
            Paint::new(n_downloaded).fg(Color::Green).bold(),
            Paint::new(n_new).fg(Color::Green).bold()
        );

        Ok(())
    }

    /// Delete the old cache and replace it with a fresh copy.
    pub fn update(&self, languages: &[String]) -> Result<()> {
        // English pages should always be extracted, so we have to add English if it is not
        // explicitly specified (when the config has `language = ["something"]` without "en").
        let mut languages = languages.to_vec();
        if !languages.iter().any(|x| x == "en") {
            languages.push("en".to_string());
        }

        // (language_dir, pages_number_before_update)
        let mut dirs_npages = HashMap::new();
        let mut all_downloaded = 0;
        let mut all_new = 0;
        let lang_dirs = languages_to_langdirs(&languages);

        for lang_dir in &lang_dirs {
            dirs_npages.insert(lang_dir.to_string(), self.list_all_vec(lang_dir)?.len());
        }

        let archives = Self::download_and_verify(languages)?;
        self.clean()?;

        for (lang_dir, mut archive) in archives {
            self.extract_lang_archive(
                &lang_dir,
                &mut archive,
                *dirs_npages.get(&lang_dir).unwrap_or(&0),
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
            fs::create_dir_all(self.0)?;
            return Ok(());
        }

        infoln!("cleaning the cache directory...");
        fs::remove_dir_all(self.0)?;
        fs::create_dir_all(self.0)?;

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

    /// Find all pages with the given name.
    pub fn find(
        &self,
        name: &str,
        languages: &[String],
        platform: Platform,
    ) -> Result<Vec<PathBuf>> {
        // https://github.com/tldr-pages/tldr/blob/main/CLIENT-SPECIFICATION.md#page-resolution

        let file = format!("{name}.md");
        let lang_dirs = languages_to_langdirs(languages);
        let mut result = vec![];

        // `common` is always searched, so we skip the search for the specified platform
        // if the user has requested only `common` (to prevent searching twice)
        if platform != Platform::Common {
            if let Some(path) = self.find_page_for(&file, &platform.to_string(), &lang_dirs) {
                result.push(path);
            }
        }

        // Fall back to `common` if the page is not found in `platform`.
        if let Some(path) = self.find_page_for(&file, "common", &lang_dirs) {
            result.push(path);
        }

        // Fall back to all other platforms if the page is not found in`platform`.
        for alt_platform in Platform::iterator() {
            // `platform` was already searched, so we can skip it here.
            if alt_platform == platform {
                continue;
            }

            if let Some(path) = self.find_page_for(&file, &alt_platform.to_string(), &lang_dirs) {
                if result.is_empty() {
                    if platform == Platform::Common {
                        warnln!(
                            "showing page from platform '{alt_platform}', \
                            because '{name}' does not exist in 'common'"
                        );
                    } else {
                        warnln!(
                            "showing page from platform '{alt_platform}', \
                            because '{name}' does not exist in '{platform}' and 'common'"
                        );
                    }
                }
                result.push(path);
            }
        }

        if result.is_empty() {
            Err(Error::new("page not found."))
        } else {
            Ok(result)
        }
    }

    /// List all available pages in `lang` for `platform`.
    fn list_dir<S>(&self, platform: &str, lang_dir: S) -> Result<Vec<OsString>>
    where
        S: AsRef<OsStr>,
    {
        if let Ok(entries) = fs::read_dir(self.0.join(lang_dir.as_ref()).join(platform)) {
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
    pub fn list_platform(&self, platform: Platform) -> Result<()> {
        let mut pages = if platform == Platform::Common {
            self.list_dir(&platform.to_string(), ENGLISH_DIR)?
        } else {
            self.list_dir(&platform.to_string(), ENGLISH_DIR)?
                .into_iter()
                .chain(self.list_dir("common", ENGLISH_DIR)?)
                .collect()
        };

        Self::print_basenames(&mut pages)
    }

    /// List all pages in `lang` and return a `Vec`.
    fn list_all_vec<S>(&self, lang_dir: S) -> Result<Vec<OsString>>
    where
        S: AsRef<OsStr>,
    {
        Ok(self
            .list_dir("linux", &lang_dir)?
            .into_iter()
            .chain(self.list_dir("osx", &lang_dir)?)
            .chain(self.list_dir("openbsd", &lang_dir)?)
            .chain(self.list_dir("windows", &lang_dir)?)
            .chain(self.list_dir("android", &lang_dir)?)
            .chain(self.list_dir("sunos", &lang_dir)?)
            .chain(self.list_dir("common", &lang_dir)?)
            .collect())
    }

    /// List all pages in English.
    pub fn list_all(&self) -> Result<()> {
        Self::print_basenames(&mut self.list_all_vec(ENGLISH_DIR)?)
    }

    /// List installed languages.
    pub fn info(&self) -> Result<()> {
        let mut n_map = BTreeMap::new();
        let mut n_total = 0;

        for lang_dir in fs::read_dir(self.0)? {
            let lang_dir = lang_dir?.file_name();
            let n = self.list_all_vec(&lang_dir)?.len();

            let str = lang_dir.to_string_lossy();
            let str = str.split_once('.').unwrap_or(("", &str)).1;

            n_map.insert(str.to_string(), n);
            n_total += n;
        }

        let mut stdout = io::stdout().lock();
        writeln!(stdout, "Installed languages:")?;

        for (lang, n) in n_map {
            writeln!(
                stdout,
                // Language codes are at most 5 characters (ll_CC).
                "{lang:5} : {}",
                Paint::new(n).fg(Color::Green).bold(),
            )?;
        }

        writeln!(
            stdout,
            "total : {} pages",
            Paint::new(n_total).fg(Color::Green).bold(),
        )?;

        Ok(())
    }

    /// Return `true` if the cache is older than `max_age`.
    pub fn is_stale(&self, max_age: Duration) -> Result<bool> {
        let since = fs::metadata(self.0)?.modified()?.elapsed().map_err(|_| {
            Error::new(
                "the system clock is not functioning correctly.\n\
                Modification time of the cache is later than the current system time.\n\
                Please fix your system clock.",
            )
        })?;

        if since > max_age {
            return Ok(true);
        }

        Ok(false)
    }
}
