use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use yansi::Color::{Green, Red};
use yansi::Paint;
use zip::ZipArchive;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::util::{self, info_end, info_start, infoln, warnln, Dedup};

const BASE_ARCHIVE_URL: &str =
    "https://raw.githubusercontent.com/tldr-pages/tldr-pages.github.io/main/assets";
const CHECKSUMS: &str =
    "https://raw.githubusercontent.com/tldr-pages/tldr-pages.github.io/main/assets/tldr.sha256sums";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), '/', env!("CARGO_PKG_VERSION"));
pub const ENGLISH_DIR: &str = "pages.en";

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

    /// Return `true` if the English pages directory exists.
    pub fn subdir_exists(&self, sd: &str) -> bool {
        self.0.join(sd).is_dir()
    }

    /// Download tldr pages archives for directories that are out of date and update the checksum file.
    fn download_and_verify(&self, languages: &[String]) -> Result<BTreeMap<String, PagesArchive>> {
        let agent = ureq::builder().user_agent(USER_AGENT).build();
        let old_sumfile_path = self.0.join("tldr.sha256sums");
        let mut langdir_archive_map = BTreeMap::new();

        infoln!("downloading 'tldr.sha256sums'...");
        let sums = agent.get(CHECKSUMS).call()?.into_string()?;
        let sum_map = Self::parse_sumfile(&sums)?;

        let old_sums = fs::read_to_string(&old_sumfile_path).unwrap_or_default();
        let old_sum_map = Self::parse_sumfile(&old_sums).unwrap_or_default();

        for lang in languages {
            let sum = sum_map.get(lang);
            // Skip nonexistent languages.
            if sum.is_none() {
                continue;
            }

            let lang_dir = format!("pages.{lang}");
            if sum == old_sum_map.get(lang) && self.subdir_exists(&lang_dir) {
                infoln!("'pages.{lang}' is up to date");
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
            let actual_sum = util::sha256_hexdigest(&archive);

            if sum != &actual_sum {
                info_end!(" {}", Paint::new("FAILED").fg(Red).bold());
                return Err(Error::new(format!(
                    "SHA256 sum mismatch!\n\
                    expected : {sum}\n\
                    got      : {actual_sum}"
                )));
            }

            info_end!(" {}", Paint::new("OK").fg(Green).bold());

            langdir_archive_map.insert(lang_dir, ZipArchive::new(Cursor::new(archive))?);
        }

        fs::create_dir_all(self.0)?;
        File::create(&old_sumfile_path)?.write_all(sums.as_bytes())?;

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
        n_existing: i32,
        all_downloaded: &mut i32,
        all_new: &mut i32,
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
            Paint::new(n_downloaded).fg(Green).bold(),
            Paint::new(n_new).fg(Green).bold()
        );

        Ok(())
    }

    /// Delete the old cache and replace it with a fresh copy.
    pub fn update(&self, languages: &[String]) -> Result<()> {
        // (language_dir, pages_number_before_update)
        let mut dirs_npages = HashMap::new();
        let mut all_downloaded = 0;
        let mut all_new = 0;

        let mut languages = languages.to_vec();
        // Sort to always download archives in alphabetical order.
        languages.sort_unstable();
        // The user can put duplicates in the config file.
        languages.dedup();
        let lang_dirs = util::languages_to_langdirs(&languages);

        for lang_dir in &lang_dirs {
            // `list_all_vec` can fail when `pages.en` is empty, hence the default of 0.
            let n = self.list_all_vec(lang_dir).map(|v| v.len()).unwrap_or(0);
            dirs_npages.insert(lang_dir.to_string(), n);
        }

        let archives = self.download_and_verify(&languages)?;

        if archives.is_empty() {
            infoln!(
                "there is nothing to do. Run 'tldr --clean-cache' if you want to force an update."
            );
            return Ok(());
        }

        for (lang_dir, mut archive) in archives {
            let lang_dir_full = self.0.join(&lang_dir);
            if lang_dir_full.is_dir() {
                fs::remove_dir_all(self.0.join(&lang_dir))?;
            }

            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            self.extract_lang_archive(
                &lang_dir,
                &mut archive,
                (*dirs_npages.get(&lang_dir).unwrap_or(&0)) as i32,
                &mut all_downloaded,
                &mut all_new,
            )?;
        }

        infoln!(
            "cache update successful (total: {} pages, {} new).",
            Paint::new(all_downloaded).fg(Green).bold(),
            Paint::new(all_new).fg(Green).bold(),
        );

        Ok(())
    }

    /// Delete the cache directory.
    pub fn clean(&self) -> Result<()> {
        if !self.0.is_dir() {
            infoln!("cache does not exist, not cleaning.");
            fs::create_dir_all(self.0)?;
            return Ok(());
        }

        infoln!("cleaning the cache directory...");
        fs::remove_dir_all(self.0)?;
        fs::create_dir_all(self.0)?;

        Ok(())
    }

    /// Find out what platforms are available.
    fn get_platforms(&self) -> Result<Vec<OsString>> {
        let mut result = vec![];

        for entry in fs::read_dir(self.0.join(ENGLISH_DIR))? {
            let entry = entry?;
            let path = entry.path();
            let platform = path.file_name().unwrap();

            result.push(platform.to_os_string());
        }

        if result.is_empty() {
            Err(Error::new(
                "'pages.en' contains no platform directories. Please run 'tldr --update'.",
            ))
        } else {
            // read_dir() order can differ across runs, so it's
            // better to sort the Vec for consistency.
            result.sort_unstable();
            Ok(result)
        }
    }

    /// Find out what platforms are available and check if the provided platform exists.
    fn get_platforms_and_check(&self, platform: &str) -> Result<Vec<OsString>> {
        let platforms = self.get_platforms()?;

        if platforms.iter().all(|x| x != platform) {
            Err(Error::new(format!(
                "platform '{platform}' does not exist.\n{} {}.",
                Paint::new("Possible values:").bold(),
                platforms.join(", ".as_ref()).to_string_lossy()
            )))
        } else {
            Ok(platforms)
        }
    }

    /// Find a page for the given platform.
    fn find_page_for<P>(&self, fname: &str, platform: P, lang_dirs: &[String]) -> Option<PathBuf>
    where
        P: AsRef<Path>,
    {
        for lang_dir in lang_dirs {
            let path = self.0.join(lang_dir).join(&platform).join(fname);

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
        languages: &mut Vec<String>,
        platform: &str,
    ) -> Result<Vec<PathBuf>> {
        // https://github.com/tldr-pages/tldr/blob/main/CLIENT-SPECIFICATION.md#page-resolution

        let platforms = self.get_platforms_and_check(platform)?;
        let file = format!("{name}.md");

        // We can't sort here - order is defined by the user.
        languages.dedup_nosort();
        let lang_dirs = util::languages_to_langdirs(languages);
        let mut result = vec![];

        // `common` is always searched, so we skip the search for the specified platform
        // if the user has requested only `common` (to prevent searching twice)
        if platform != "common" {
            if let Some(path) = self.find_page_for(&file, platform, &lang_dirs) {
                result.push(path);
            }
        }

        // Fall back to `common` if the page is not found in `platform`.
        if let Some(path) = self.find_page_for(&file, "common", &lang_dirs) {
            result.push(path);
        }

        // Fall back to all other platforms if the page is not found in `platform`.
        for alt_platform in platforms {
            // `platform` and `common` were already searched, so we can skip them here.
            if alt_platform == platform || alt_platform == "common" {
                continue;
            }

            if let Some(path) = self.find_page_for(&file, &alt_platform, &lang_dirs) {
                if result.is_empty() {
                    let alt_platform = alt_platform.to_string_lossy();

                    if platform == "common" {
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

        Ok(result)
    }

    /// List all available pages in `lang` for `platform`.
    fn list_dir<P, Q>(&self, platform: P, lang_dir: Q) -> Result<Vec<OsString>>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        match fs::read_dir(self.0.join(lang_dir.as_ref()).join(platform)) {
            Ok(entries) => {
                let entries = entries.map(|res| res.map(|ent| ent.file_name()));
                Ok(entries.collect::<io::Result<Vec<OsString>>>()?)
            }
            // If the directory does not exist, return an empty Vec instead of an error
            // (some platform directories do not exist in some translations).
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    fn print_basenames(mut pages: Vec<OsString>) -> Result<()> {
        // Show pages in alphabetical order.
        pages.sort_unstable();
        // There are pages with the same name across multiple platforms.
        // Listing these multiple times makes no sense.
        pages.dedup();

        let mut stdout = BufWriter::new(io::stdout().lock());

        for page in pages {
            let page = page.to_string_lossy();
            let page = page.strip_suffix(".md").unwrap_or(&page);

            writeln!(stdout, "{page}")?;
        }

        Ok(stdout.flush()?)
    }

    /// List all pages in English for `platform` and common.
    pub fn list_for(&self, platform: &str) -> Result<()> {
        self.get_platforms_and_check(platform)?;

        let pages = if platform == "common" {
            self.list_dir(platform, ENGLISH_DIR)?
        } else {
            self.list_dir(platform, ENGLISH_DIR)?
                .into_iter()
                .chain(self.list_dir("common", ENGLISH_DIR)?)
                .collect()
        };

        Self::print_basenames(pages)
    }

    /// List all pages in `lang` and return a `Vec`.
    fn list_all_vec<S>(&self, lang_dir: S) -> Result<Vec<OsString>>
    where
        S: AsRef<Path>,
    {
        let mut result = vec![];

        for platform in self.get_platforms()? {
            result.append(&mut self.list_dir(&platform, &lang_dir)?);
        }

        Ok(result)
    }

    /// List all pages in English.
    pub fn list_all(&self) -> Result<()> {
        Self::print_basenames(self.list_all_vec(ENGLISH_DIR)?)
    }

    /// List platforms (used in shell completions).
    pub fn list_platforms(&self) -> Result<()> {
        let platforms = self.get_platforms()?.join("\n".as_ref());
        writeln!(io::stdout(), "{}", platforms.to_string_lossy())?;
        Ok(())
    }

    /// List languages (used in shell completions).
    pub fn list_languages(&self) -> Result<()> {
        let languages = fs::read_dir(self.0)?
            .filter(|res| res.is_ok() && res.as_ref().unwrap().path().is_dir())
            .map(|res| res.unwrap().file_name())
            .collect::<Vec<OsString>>();
        let mut stdout = io::stdout().lock();

        for lang in languages {
            let lang = lang.to_string_lossy();
            let lang = lang.strip_prefix("pages.").unwrap_or(&lang);

            writeln!(stdout, "{lang}")?;
        }

        Ok(())
    }

    /// Show cache information.
    pub fn info(&self, cfg: &Config) -> Result<()> {
        let mut n_map = BTreeMap::new();
        let mut n_total = 0;

        for lang_dir in fs::read_dir(self.0)? {
            let lang_dir = lang_dir?;
            if !lang_dir.path().is_dir() {
                continue;
            }
            let lang_dir = lang_dir.file_name();
            let n = self.list_all_vec(&lang_dir)?.len();

            let lang = lang_dir.to_string_lossy();
            let lang = lang.strip_prefix("pages.").unwrap_or(&lang);

            n_map.insert(lang.to_string(), n);
            n_total += n;
        }

        let mut stdout = io::stdout().lock();
        let age = self.age()?.as_secs();

        writeln!(
            stdout,
            "Cache: {} (last update: {} ago)",
            Paint::new(self.0.display()).fg(Red),
            Paint::new(util::duration_fmt(age)).fg(Green).bold()
        )?;

        if cfg.cache.auto_update {
            let age_diff = cfg.cache_max_age().as_secs() - age;

            writeln!(
                stdout,
                "Automatic update in {}",
                Paint::new(util::duration_fmt(age_diff)).fg(Green).bold()
            )?;
        } else {
            writeln!(stdout, "Automatic updates are disabled")?;
        }

        writeln!(stdout, "Installed languages:")?;

        for (lang, n) in n_map {
            writeln!(
                stdout,
                // Language codes are at most 5 characters (ll_CC).
                "{lang:5} : {}",
                Paint::new(n).fg(Green).bold(),
            )?;
        }

        writeln!(
            stdout,
            "total : {} pages",
            Paint::new(n_total).fg(Green).bold(),
        )?;

        Ok(())
    }

    /// Get the age of the cache.
    pub fn age(&self) -> Result<Duration> {
        let sumfile = self.0.join("tldr.sha256sums");
        let metadata = if sumfile.is_file() {
            fs::metadata(&sumfile)
        } else {
            // The sumfile is not available, fall back to the base directory.
            fs::metadata(self.0)
        }?;

        metadata.modified()?.elapsed().map_err(|_| {
            Error::new(
                "the system clock is not functioning correctly.\n\
                Modification time of the cache is later than the current system time.\n\
                Please fix your system clock.",
            )
        })
    }
}
