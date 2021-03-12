use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use util::PathEncodingError;

use super::Error;

/// Copy `src` to `tgt`, recursively if needed.
pub fn copy(src: &Path, tgt: &Path) -> Result<()> {
    if src.is_symlink() {
        let link_tgt = fs::read_link(src)?;
        symlink(&link_tgt, tgt)?;
    } else if src.is_file() {
        fs::copy(src, tgt)?;
    } else if src.is_dir() {
        cp_dir(src, tgt, src, tgt)?;
    } else {
        return Err(
            Error::UnknownPathType(src.to_str().ok_or(PathEncodingError)?.to_owned()).into(),
        );
    }
    Ok(())
}

fn cp_dir(src_root: &Path, tgt_root: &Path, src: &Path, tgt: &Path) -> Result<()> {
    fs::create_dir_all(tgt)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_entry = entry.path();
        let tgt_entry = tgt.join(entry.file_name());
        if ty.is_symlink() {
            let orig_link_tgt = fs::read_link(&src_entry)?;
            let new_link_tgt = resolve_new_link_tgt(src_root, tgt_root, orig_link_tgt)?;
            symlink(&new_link_tgt, &tgt_entry)?;
        } else if ty.is_dir() {
            cp_dir(src_root, tgt_root, &src_entry, &tgt_entry)?;
        } else if ty.is_file() {
            fs::copy(&src_entry, &tgt_entry)?;
        } else {
            return Err(Error::UnknownPathType(
                entry.path().to_str().ok_or(PathEncodingError)?.to_owned(),
            )
            .into());
        }
    }
    Ok(())
}

/// If link is internal to `src_root`, create a new internal link in `tgt_root`.
/// O/w, just link to the same external target.
fn resolve_new_link_tgt(
    src_root: &Path,
    tgt_root: &Path,
    orig_link_tgt: PathBuf,
) -> Result<PathBuf> {
    if orig_link_tgt.starts_with(src_root) {
        Ok(tgt_root.join(orig_link_tgt.strip_prefix(src_root)?))
    } else {
        Ok(orig_link_tgt)
    }
}

/// Symlink the given `link` to `tgt`; works for unix and windows.
pub fn symlink(tgt: &Path, link: &Path) -> Result<()> {
    #[cfg(unix)]
    std::os::unix::fs::symlink(tgt, link)?;

    #[cfg(windows)]
    if tgt.is_dir() {
        std::os::windows::fs::link_dir(tgt, link)?;
    } else {
        std::os::windows::fs::link_file(tgt, link)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tempfile::tempdir;
    #[test]
    fn test_copy_file() -> Result<()> {
        use std::fs;
        use std::io::Read;

        let dir = tempdir()?;
        let src = dir.path().join("src");
        fs::write(&src, "text to copy")?;

        let tgt = dir.path().join("tgt");

        copy(&src, &tgt)?;

        let mut buf = String::with_capacity(16);
        let mut f = fs::File::open(&tgt)?;
        f.read_to_string(&mut buf)?;

        assert!(tgt.exists());
        assert_eq!(buf, "text to copy");

        Ok(())
    }
    #[test]
    fn test_copy_dir() -> Result<()> {
        use std::fs;
        use std::io::Read;

        let dir = tempdir()?;
        let src = dir.path().join("src/prefix/dir");
        fs::create_dir_all(&src)?;
        let src_subdir = src.join("subdir");
        fs::create_dir(&src_subdir)?;
        let file = src_subdir.join("file");
        fs::write(&file, "text to copy")?;

        let dir_link = src.join("dir_link");
        symlink(&src_subdir, &dir_link)?;

        let file_link = src.join("file_link");
        symlink(&file, &file_link)?;

        let external_link = src.join("external_link");
        symlink("/dev/null".as_ref(), &external_link)?;

        let tgt = dir.path().join("tgt");

        copy(&src, &tgt)?;

        assert!(tgt.join("subdir").is_dir());
        assert!(tgt.join("subdir/file").exists());

        let tgt_dir_link = tgt.join("dir_link");
        assert!(tgt_dir_link.is_symlink());
        assert_eq!(fs::read_link(&tgt_dir_link)?, tgt.join("subdir"));

        let tgt_file_link = tgt.join("file_link");
        assert!(tgt_file_link.is_symlink());
        assert_eq!(fs::read_link(&tgt_file_link)?, tgt.join("subdir/file"));

        let tgt_external_link = tgt.join("external_link");
        assert!(tgt_external_link.is_symlink());
        assert_eq!(&fs::read_link(&tgt_external_link)?, &Path::new("/dev/null"));

        let mut buf = String::with_capacity(16);
        let mut f = fs::File::open(&tgt.join("subdir/file"))?;
        f.read_to_string(&mut buf)?;

        assert_eq!(buf, "text to copy");

        Ok(())
    }
}
