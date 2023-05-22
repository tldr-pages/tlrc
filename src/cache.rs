use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::time::Duration;

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

    /// Extract `dir` from `archive`.
    fn extract_dir(
        &self,
        archive: &mut ZipArchive<Cursor<Vec<u8>>>,
        files: &[String],
        dir: &str,
    ) -> Result<()> {
        infoln!("extracting: '{dir}'...");

        for f in files {
            // Skip directory entries, files that are not in a directory (we want only pages)
            // and files that are not in the specified directory.
            if f.ends_with('/') || !f.contains('/') || !f.starts_with(&format!("{dir}/")) {
                continue;
            }

            let mut page = String::new();
            archive
                .by_name(f)
                .unwrap()
                .read_to_string(&mut page)
                .unwrap();

            let path = self.0.join(f);
            fs::create_dir_all(path.parent().unwrap())?;
            write!(File::create(path)?, "{page}")?;
        }

        Ok(())
    }

    /// Delete the old cache and replace it with a fresh copy.
    pub fn update(&self, languages_to_download: &[String]) -> Result<()> {
        let mut archive = ZipArchive::new(Cursor::new(Self::download()?))?;
        let files: Vec<String> = archive.file_names().map(String::from).collect();
        let langdirs = languages_to_langdirs(languages_to_download);

        self.clean()?;

        // Always extract English pages, even when not specified in the config.
        if !languages_to_download.contains(&"en".to_string()) {
            self.extract_dir(&mut archive, &files, "pages")?;
        }

        for langdir in langdirs {
            // Skip invalid languages.
            if !files.contains(&format!("{langdir}/")) {
                continue;
            }

            self.extract_dir(&mut archive, &files, &langdir)?;
        }

        infoln!("cache update successful.");
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

    fn find_page(
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
                self.find_page(&page_file, &platform.to_string(), &language_dirs)
            {
                return Ok(page_path);
            }
        }

        if let Some(page_path) = self.find_page(&page_file, "common", &language_dirs) {
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
                self.find_page(&page_file, &alt_platform.to_string(), &language_dirs)
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

    /// List all available pages in English for `platform`.
    fn list_dir(&self, platform: &str) -> Result<Vec<PathBuf>> {
        Ok(
            fs::read_dir(format!("{}/pages/{platform}", self.0.display()))?
                .map(|res| res.map(|e| e.path()))
                .collect::<StdResult<Vec<PathBuf>, io::Error>>()?,
        )
    }

    fn print_basenames(entries: &[PathBuf]) -> Result<()> {
        let mut pages: Vec<String> = entries
            .iter()
            .map(|x| Path::new(x.file_stem().unwrap()).display().to_string())
            .collect();

        pages.sort();
        pages.dedup();

        Ok(writeln!(io::stdout(), "{}", pages.join("\n"))?)
    }

    /// List all pages in `platform` and common.
    pub fn list_platform(&self, platform: &Platform) -> Result<()> {
        let entries: Vec<PathBuf> = if platform == &Platform::Common {
            self.list_dir(&platform.to_string())?
        } else {
            self.list_dir(&platform.to_string())?
                .into_iter()
                .chain(self.list_dir("common")?.into_iter())
                .collect()
        };

        Self::print_basenames(&entries)
    }

    /// List all pages.
    pub fn list_all(&self) -> Result<()> {
        let entries: Vec<PathBuf> = self
            .list_dir("linux")?
            .into_iter()
            .chain(self.list_dir("osx")?)
            .chain(self.list_dir("windows")?)
            .chain(self.list_dir("android")?)
            .chain(self.list_dir("sunos")?)
            .chain(self.list_dir("common")?)
            .collect();

        Self::print_basenames(&entries)
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
