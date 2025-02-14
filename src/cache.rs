use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufWriter, Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use once_cell::unsync::OnceCell;
use ureq::tls::{RootCerts, TlsConfig};
use yansi::Paint;
use zip::ZipArchive;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::util::{self, info_end, info_start, infoln, warnln, Dedup};

pub const ENGLISH_DIR: &str = "pages.en";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), '/', env!("CARGO_PKG_VERSION"));

type PagesArchive = ZipArchive<Cursor<Vec<u8>>>;

pub struct Cache<'a> {
    dir: &'a Path,
    platforms: OnceCell<Vec<OsString>>,
    age: OnceCell<Duration>,
}

impl<'a> Cache<'a> {
    pub fn new(dir: &'a Path) -> Self {
        Self {
            dir,
            platforms: OnceCell::new(),
            age: OnceCell::new(),
        }
    }

    /// Get the default path to the cache.
    pub fn locate() -> PathBuf {
        dirs::cache_dir().unwrap().join(env!("CARGO_PKG_NAME"))
    }

    /// Return `true` if the specified subdirectory exists in the cache.
    pub fn subdir_exists(&self, sd: &str) -> bool {
        self.dir.join(sd).is_dir()
    }

    /// Send a GET request with the provided agent and return the response body.
    fn get_asset(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>> {
        info_start!("downloading '{}'... ", url.split('/').last().unwrap());

        let mut resp = match agent.get(url).call() {
            Ok(r) => r,
            Err(e) => {
                info_end!("{}", "FAILED".red().bold());
                return Err(e.into());
            }
        };
        let body = resp.body_mut();
        let bytes = match body.with_config().limit(1_000_000_000).read_to_vec() {
            Ok(v) => v,
            Err(e) => {
                info_end!("{}", "FAILED".red().bold());
                return Err(e.into());
            }
        };

        #[allow(clippy::cast_precision_loss)]
        let dl_kib = bytes.len() as f64 / 1024.0;
        if dl_kib < 1024.0 {
            info_end!("{:.02} KiB", dl_kib.green().bold());
        } else {
            info_end!("{:.02} MiB", (dl_kib / 1024.0).green().bold());
        }

        Ok(bytes)
    }

    /// Download tldr pages archives for directories that are out of date and update the checksum file.
    fn download_and_verify(
        &self,
        mirror: &str,
        languages: &[String],
    ) -> Result<BTreeMap<String, PagesArchive>> {
        let agent = ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(Duration::from_secs(5)))
            .tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .into();

        let sums = Self::get_asset(&agent, &format!("{mirror}/tldr.sha256sums"))?;
        let sums_str = String::from_utf8_lossy(&sums);
        let sum_map = Self::parse_sumfile(&sums_str)?;

        let old_sumfile_path = self.dir.join("tldr.sha256sums");
        let old_sums = fs::read_to_string(&old_sumfile_path).unwrap_or_default();
        let old_sum_map = Self::parse_sumfile(&old_sums).unwrap_or_default();

        let mut langdir_archive_map = BTreeMap::new();

        for lang in languages {
            let lang = &**lang;
            let Some(sum) = sum_map.get(lang) else {
                // Skip nonexistent languages.
                continue;
            };

            let lang_dir = format!("pages.{lang}");
            if Some(sum) == old_sum_map.get(lang) && self.subdir_exists(&lang_dir) {
                infoln!("'pages.{lang}' is up to date");
                continue;
            }

            let archive = Self::get_asset(&agent, &format!("{mirror}/tldr-pages.{lang}.zip"))?;
            info_start!("validating sha256sums... ");
            let actual_sum = util::sha256_hexdigest(&archive);

            if sum != &actual_sum {
                info_end!("{}", "FAILED".red().bold());
                return Err(Error::new(format!(
                    "SHA256 sum mismatch!\n\
                    expected : {sum}\n\
                    got      : {actual_sum}"
                )));
            }

            info_end!(" {}", "OK".green().bold());

            langdir_archive_map.insert(lang_dir, ZipArchive::new(Cursor::new(archive))?);
        }

        fs::create_dir_all(self.dir)?;
        File::create(&old_sumfile_path)?.write_all(&sums)?;

        Ok(langdir_archive_map)
    }

    fn parse_sumfile(s: &str) -> Result<HashMap<&str, &str>> {
        // Subtract 3, because 3 lines are skipped in the loop.
        let mut map = HashMap::with_capacity(s.lines().count().saturating_sub(3));

        for l in s.lines() {
            // The file looks like this:
            // sha256sum     tldr-pages.lang.zip
            // sha256sum     tldr-pages.lang.zip
            // ...

            let mut spl = l.split_whitespace();
            let sum = spl.next().ok_or_else(Error::parse_sumfile)?;
            let path = spl.next().ok_or_else(Error::parse_sumfile)?;

            // Skip other files, the full archive, and the old English archive.
            // This map is used to detect languages available to download.
            // Not skipping index.json makes "json" a language.
            // Not skipping archives without a language in the filename makes "zip" a language.
            if !path.ends_with("zip")
                || path.ends_with("tldr.zip")
                || path.ends_with("tldr-pages.zip")
            {
                continue;
            }

            let lang = path.split('.').nth(1).ok_or_else(Error::parse_sumfile)?;
            map.insert(lang, sum);
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
        info_start!("extracting '{lang_dir}'... ");

        let mut n_downloaded = 0;

        for i in 0..archive.len() {
            let mut zipfile = archive.by_index(i)?;
            let Some(fname) = zipfile.enclosed_name() else {
                warnln!(
                    "found an unsafe path in the zip archive: '{}', ignoring it",
                    zipfile.name()
                );
                continue;
            };

            // Skip files that are not in a directory (we want only pages).
            if zipfile.is_file() && fname.parent() == Some(Path::new("")) {
                continue;
            }

            let path = self.dir.join(lang_dir).join(&fname);

            if zipfile.is_dir() {
                fs::create_dir_all(&path)?;
                continue;
            }

            let mut file = File::create(&path)?;
            io::copy(&mut zipfile, &mut file)?;

            n_downloaded += 1;
        }

        let n_new = n_downloaded - n_existing;
        *all_downloaded += n_downloaded;
        *all_new += n_new;

        info_end!(
            "{} pages, {} new",
            n_downloaded.green().bold(),
            n_new.green().bold()
        );

        Ok(())
    }

    /// Delete the old cache and replace it with a fresh copy.
    pub fn update(&self, mirror: &str, languages: &mut Vec<String>) -> Result<()> {
        // Sort to always download archives in alphabetical order.
        languages.sort_unstable();
        // The user can put duplicates in the config file.
        languages.dedup();

        let archives = self.download_and_verify(mirror, languages)?;

        if archives.is_empty() {
            infoln!(
                "there is nothing to do. Run 'tldr --clean-cache' if you want to force an update."
            );
            return Ok(());
        }

        let mut all_downloaded = 0;
        let mut all_new = 0;

        for (lang_dir, mut archive) in archives {
            // `list_all_vec` can fail when `pages.en` is empty, hence the default of 0.
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let n_existing = self.list_all_vec(&lang_dir).map(|v| v.len()).unwrap_or(0) as i32;

            let lang_dir_full = self.dir.join(&lang_dir);
            if lang_dir_full.is_dir() {
                fs::remove_dir_all(&lang_dir_full)?;
            }

            if let Err(e) = self.extract_lang_archive(
                &lang_dir,
                &mut archive,
                n_existing,
                &mut all_downloaded,
                &mut all_new,
            ) {
                info_end!("{}", "FAILED".red().bold());
                return Err(e);
            }
        }

        infoln!(
            "cache update successful (total: {} pages, {} new).",
            all_downloaded.green().bold(),
            all_new.green().bold(),
        );

        Ok(())
    }

    /// Delete the cache directory.
    pub fn clean(&self) -> Result<()> {
        if !self.dir.is_dir() {
            infoln!("cache does not exist, not cleaning.");
            fs::create_dir_all(self.dir)?;
            return Ok(());
        }

        infoln!("cleaning the cache directory...");
        fs::remove_dir_all(self.dir)?;
        fs::create_dir_all(self.dir)?;

        Ok(())
    }

    /// Find out what platforms are available.
    fn get_platforms(&self) -> Result<&[OsString]> {
        self.platforms
            .get_or_try_init(|| {
                let mut result = vec![];

                for entry in fs::read_dir(self.dir.join(ENGLISH_DIR))? {
                    let entry = entry?;
                    let path = entry.path();
                    let platform = path.file_name().unwrap();

                    result.push(platform.to_os_string());
                }

                if result.is_empty() {
                    Err(Error::messed_up_cache(
                        "'pages.en' contains no platform directories.",
                    ))
                } else {
                    // read_dir() order can differ across runs, so it's
                    // better to sort the Vec for consistency.
                    result.sort_unstable();
                    Ok(result)
                }
            })
            .map(Vec::as_slice)
    }

    /// Find out what platforms are available and check if the provided platform exists.
    fn get_platforms_and_check(&self, platform: &str) -> Result<&[OsString]> {
        let platforms = self.get_platforms()?;

        if platforms.iter().all(|x| x != platform) {
            Err(Error::new(format!(
                "platform '{platform}' does not exist.\n{} {}.",
                "Possible values:".bold(),
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
            let path = self.dir.join(lang_dir).join(&platform).join(fname);

            if path.is_file() {
                return Some(path);
            }
        }

        None
    }

    /// Find all pages with the given name.
    pub fn find(&self, name: &str, languages: &[String], platform: &str) -> Result<Vec<PathBuf>> {
        // https://github.com/tldr-pages/tldr/blob/main/CLIENT-SPECIFICATION.md#page-resolution

        let platforms = self.get_platforms_and_check(platform)?;
        let file = format!("{name}.md");

        let mut result = vec![];
        let mut lang_dirs: Vec<String> = languages.iter().map(|x| format!("pages.{x}")).collect();
        // We can't sort here - order is defined by the user.
        lang_dirs.dedup_nosort();

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

            if let Some(path) = self.find_page_for(&file, alt_platform, &lang_dirs) {
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
        match fs::read_dir(self.dir.join(lang_dir.as_ref()).join(platform)) {
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
        if pages.is_empty() {
            return Err(Error::messed_up_cache(
                "no pages found, but the 'pages.en' directory exists.",
            ));
        }

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
        // This is here just to check if the platform exists.
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
            result.append(&mut self.list_dir(platform, &lang_dir)?);
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
        let languages = fs::read_dir(self.dir)?
            .filter(|res| res.is_ok() && res.as_ref().unwrap().path().is_dir())
            .map(|res| res.unwrap().file_name());
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

        for lang_dir in fs::read_dir(self.dir)? {
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
            self.dir.display().red(),
            util::duration_fmt(age).green().bold()
        )?;

        if cfg.cache.auto_update {
            let max_age = cfg.cache_max_age().as_secs();
            if max_age > age {
                let age_diff = max_age - age;

                writeln!(
                    stdout,
                    "Automatic update in {}",
                    util::duration_fmt(age_diff).green().bold()
                )?;
            }
        } else {
            writeln!(stdout, "Automatic updates are disabled")?;
        }

        writeln!(stdout, "Installed languages:")?;

        for (lang, n) in n_map {
            writeln!(
                stdout,
                // Language codes are at most 5 characters (ll_CC).
                "{lang:5} : {}",
                n.green().bold(),
            )?;
        }

        writeln!(stdout, "total : {} pages", n_total.green().bold())?;

        Ok(())
    }

    /// Get the age of the cache.
    pub fn age(&self) -> Result<Duration> {
        self.age
            .get_or_try_init(|| {
                let sumfile = self.dir.join("tldr.sha256sums");
                let metadata = if sumfile.is_file() {
                    fs::metadata(&sumfile)
                } else {
                    // The sumfile is not available, fall back to the base directory.
                    fs::metadata(self.dir)
                }?;

                metadata.modified()?.elapsed().map_err(|_| {
                    Error::new(
                        "the system clock is not functioning correctly.\n\
                        Modification time of the cache is later than the current system time.\n\
                        Please fix your system clock.",
                    )
                })
            })
            .copied()
    }
}
