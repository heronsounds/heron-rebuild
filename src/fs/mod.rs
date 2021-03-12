use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{Context, Result};

use util::PathEncodingError;

/// Utility fns
mod ops;

/// Defines fns for creating common paths in the output directory
mod paths;

/// Dealing with the branchpoints.txt file
mod branchpoints_txt;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Path is neither file nor dir: {0}")]
    UnknownPathType(String),
    #[error("Specified output directory \"{0}\" is not a directory")]
    NotDirectory(String),
    #[error("Can't perform IO operation: \"{0}\" is not whitelisted")]
    NotWhitelisted(String),
    #[error("Invalid branchpoints.txt file")]
    InvalidBranchpointsFile,
}

/// All file operations in the crate should go through this struct.
///
/// All destructive operations check that the path in question is a child of the
/// single whitelisted prefix (the output dir), otherwise they will not be performed.
/// Note that code blocks in the config file can break this rule; it is up to the user
/// to make sure that the code there doesn't have unintended consequences.
#[derive(Debug)]
pub struct Fs {
    /// The directory we are allowed to modify
    output_prefix: PathBuf,
    /// if true, prevents all destructive operations
    dry_run: bool,
}

impl Fs {
    /// Create a new `Fs` with the given output directory.
    pub fn new(output_prefix: &Path, dry_run: bool) -> Self {
        Self {
            output_prefix: output_prefix.to_path_buf(),
            dry_run,
        }
    }

    /// Set the `dry_run` variable to true or false.
    /// If true, no destructive operations will be performed.
    pub fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }

    /// Check whether output dir exists, and create it if not.
    pub fn ensure_output_dir_exists(&mut self, verbose: bool) -> Result<()> {
        if !self.output_prefix.exists() {
            if self.dry_run {
                eprintln!(
                    "Dry run. Not creating output directory {:?}",
                    self.output_prefix
                );
            } else {
                eprintln!(
                    "Output directory {:?} doesn't exist. Creating.",
                    self.output_prefix
                );
                fs::create_dir_all(&self.output_prefix).context("creating output directory")?;
            }
        } else if !self.output_prefix.is_dir() {
            return Err(Error::NotDirectory(
                self.output_prefix
                    .to_str()
                    .ok_or(PathEncodingError)?
                    .to_string(),
            )
            .into());
        } else if verbose {
            eprintln!(
                "Output directory {:?} already exists. Not creating.",
                self.output_prefix
            );
        }

        self.output_prefix = self.output_prefix.canonicalize()?;
        Ok(())
    }

    /// Check if path exists on disk.
    pub fn exists<T: AsRef<Path>>(&self, path: T) -> bool {
        let path = path.as_ref();
        path.exists() || path.is_symlink()
    }

    /// Check if path exists and is a directory.
    pub fn is_dir<T: AsRef<Path>>(&self, path: T) -> Result<bool> {
        let path = path.as_ref();
        if path.is_dir() || (path.is_symlink() && path.canonicalize()?.is_dir()) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Create a directory (uses `std::fs::create_dir_all`, so an entire tree of dirs can be created).
    pub fn create_dir<T: AsRef<Path>>(&self, path: T) -> Result<()> {
        let path = path.as_ref();
        self.check_whitelist(path)?;
        fs::create_dir_all(path).context("creating dir")?;
        Ok(())
    }

    /// Create parent directory of a given path.
    pub fn create_parent_dir<T: AsRef<Path>>(&self, path: T) -> Result<()> {
        let path = path.as_ref();
        let parent = path.parent().unwrap();
        self.check_whitelist(parent)?;
        fs::create_dir_all(parent).context("creating parent dir")?;
        Ok(())
    }

    /// Create a file, and return a writable `File` handle.
    pub fn create_file<T: AsRef<Path>>(&self, path: T) -> Result<fs::File> {
        let path = path.as_ref();
        self.check_whitelist(path)?;
        let f = fs::File::create(path).context("creating file")?;
        Ok(f)
    }

    /// Write entire str to a file.
    pub fn write_file<T: AsRef<Path>>(&self, path: T, text: &str) -> Result<()> {
        let path = path.as_ref();
        self.check_whitelist(path)?;
        fs::write(path, text).context("writing file")?;
        Ok(())
    }

    /// Delete a file.
    pub fn delete_file<T: AsRef<Path>>(&self, path: T) -> Result<()> {
        let path = path.as_ref();
        self.check_whitelist(path)?;
        fs::remove_file(path).context("deleting file")?;
        Ok(())
    }

    /// Recursively delete a directory.
    pub fn delete_dir<T: AsRef<Path>>(&self, path: T) -> Result<()> {
        let path = path.as_ref();
        self.check_whitelist(path)?;
        fs::remove_dir_all(path).context("deleting dir")?;
        Ok(())
    }

    /// Symlink `symlink` to `tgt`.
    pub fn symlink<T: AsRef<Path>, U: AsRef<Path>>(&self, tgt: T, symlink: U) -> Result<()> {
        let (tgt, symlink) = (tgt.as_ref(), symlink.as_ref());
        self.check_whitelist(symlink)?;
        ops::symlink(tgt, symlink)
            .with_context(|| format!("symlinking {:?} to {:?}", symlink, tgt))?;
        Ok(())
    }

    /// Copy `src` to `tgt`, recursively if `src` is a directory.
    pub fn copy<T: AsRef<Path>, U: AsRef<Path>>(&self, src: T, tgt: U) -> Result<()> {
        let (src, tgt) = (src.as_ref(), tgt.as_ref());
        self.check_whitelist(tgt)?;
        ops::copy(src, tgt).context("copying file")?;
        Ok(())
    }

    /// Read entire file into a String.
    pub fn read_to_buf<T: AsRef<Path>>(&self, path: T, strbuf: &mut String) -> Result<()> {
        use std::io::Read;
        let path = path.as_ref();
        strbuf.clear();
        let cap = fs::metadata(path)?.len() as usize;
        if cap > strbuf.len() {
            strbuf.reserve(cap - strbuf.len());
        }
        let mut f = fs::File::open(path)?;
        f.read_to_string(strbuf)?;
        Ok(())
    }

    /// List entries in a directory
    pub fn read_dir<T: AsRef<Path>>(&self, path: T) -> Result<fs::ReadDir, io::Error> {
        fs::read_dir(path)
    }

    fn is_whitelisted<T: AsRef<Path>>(&self, path: T) -> bool {
        let path = path.as_ref();
        if path.starts_with(&self.output_prefix) {
            return true;
        }
        false
    }

    fn check_whitelist(&self, path: &Path) -> Result<()> {
        if self.dry_run || !self.is_whitelisted(path) {
            Err(Error::NotWhitelisted(path.to_str().ok_or(PathEncodingError)?.to_owned()).into())
        } else {
            Ok(())
        }
    }
}
