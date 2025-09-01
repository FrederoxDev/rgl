use anyhow::{anyhow, bail, Context, Result};
use dashmap::DashMap;
use dunce::canonicalize;
use jsonc_parser::ParseOptions;
use rayon::prelude::*;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::LazyLock,
    time::SystemTime,
};

fn copy_dir_impl(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    fs::read_dir(from)?
        .par_bridge()
        .try_for_each(|entry| -> Result<()> {
            let entry = entry?;
            let path = entry.path();
            let to = to.join(entry.file_name());
            if path.is_dir() {
                copy_dir_impl(&path, &to)?;
            } else {
                fs::copy(path, to)?;
            }
            Ok(())
        })
}

pub fn copy_dir(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    copy_dir_impl(from, to).with_context(|| {
        format!(
            "Failed to copy directory\n\
             <yellow> >></> From: {}\n\
             <yellow> >></> To: {}",
            from.display(),
            to.display(),
        )
    })
}

fn empty_dir_impl(path: &Path) -> Result<()> {
    rimraf(path).map_err(|e| anyhow!("{}", e.root_cause()))?;
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn empty_dir(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    empty_dir_impl(path).with_context(|| {
        format!(
            "Failed to empty directory\n\
             <yellow> >></> Path: {}",
            path.display(),
        )
    })
}

pub fn read_json<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let path = path.as_ref();
    let inner = || -> Result<T> {
        let data = fs::read_to_string(path)?;
        let value = jsonc_parser::parse_to_serde_value(&data, &ParseOptions::default())?
            .unwrap_or_default();
        let json = serde_json::from_value(value)?;
        Ok(json)
    };
    inner().with_context(|| {
        format!(
            "Failed to read JSON file\n\
             <yellow> >></> Path: {}",
            path.display()
        )
    })
}

pub fn rimraf(path: impl AsRef<Path>) -> Result<()> {
    fn remove_entry(path: &Path, metadata: &fs::Metadata) -> Result<()> {
        let rm = if cfg!(windows) && metadata.is_symlink() {
            fs::remove_dir
        } else {
            fs::remove_file
        };
        if let Err(e) = rm(&path) {
            match e.kind() {
                io::ErrorKind::PermissionDenied => {
                    let mut perm = metadata.permissions();
                    perm.set_readonly(false);
                    fs::set_permissions(path, perm)?;
                    rm(&path)?;
                }
                _ => bail!(e),
            }
        }
        Ok(())
    }

    fn rimraf_impl(path: &Path) -> Result<()> {
        fs::read_dir(path)?
            .par_bridge()
            .try_for_each(|entry| -> Result<()> {
                let entry = entry?;
                let path = entry.path();
                let metadata = entry.metadata()?;
                if metadata.is_dir() {
                    rimraf_impl(&path)
                } else {
                    remove_entry(&path, &metadata)
                }
            })?;
        fs::remove_dir(path)?;
        Ok(())
    }

    let path = path.as_ref();
    let metadata = match path.symlink_metadata() {
        Ok(val) => val,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => bail!(e),
    };
    if metadata.is_dir() {
        rimraf_impl(path).with_context(|| {
            format!(
                "Failed to remove directory\n\
                 <yellow> >></> Path: {}",
                path.display()
            )
        })
    } else {
        remove_entry(path, &metadata).with_context(|| {
            format!(
                "Failed to remove\n\
                 <yellow> >></> Path: {}",
                path.display()
            )
        })
    }
}

/// Checks if directory exists and is not empty
pub fn is_dir_empty(path: &Path) -> Result<bool> {
    Ok(!path.is_dir() || path.read_dir()?.next().is_none())
}

pub fn set_modified_time(path: impl AsRef<Path>, time: SystemTime) -> Result<()> {
    let inner = || {
        fs::File::options()
            .write(true)
            .open(&path)?
            .set_modified(time)
    };
    inner().with_context(|| {
        format!(
            "Failed to set modified time\n\
             <yellow> >></> Path: {}",
            path.as_ref().display(),
        )
    })
}

#[cfg(unix)]
fn symlink_impl(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::unix;
    unix::fs::symlink(canonicalize(from)?, to)
}

#[cfg(windows)]
fn symlink_impl(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows;
    windows::fs::symlink_dir(canonicalize(from)?, to).map_err(|e| match e.raw_os_error() {
        Some(1314) => io::Error::other(
            "A required privilege is not held by the client. (os error 1314)\n\
             <blue>[?]</> Try enabling developer mode in Windows settings or run the terminal as an administrator",
        ),
        _ => e,
    })
}

pub fn symlink(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    symlink_impl(from, to).with_context(|| {
        format!(
            "Failed to create symlink\n\
             <yellow> >></> From: {}\n\
             <yellow> >></> To: {}",
            from.display(),
            to.display()
        )
    })
}

pub fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    let path = path.as_ref();
    fs::write(path, contents).with_context(|| {
        format!(
            "Failed to write file\n\
             <yellow> >></> Path: {}",
            path.display()
        )
    })
}

pub fn write_json<T>(path: impl AsRef<Path>, data: &T) -> Result<()>
where
    T: serde::Serialize,
{
    let path = path.as_ref();
    let inner = || -> Result<()> {
        let data = serde_json::to_string_pretty(data)?;
        write_file(path, data + "\n")?;
        Ok(())
    };
    inner().with_context(|| {
        format!(
            "Failed to write JSON file\n\
             <yellow> >></> Path: {}",
            path.display()
        )
    })
}

/// Sync target directory with source directory.
///
/// **Not thread-safe!** Uses a global cache internally. Do NOT call in parallel or from async code.
pub fn sync_dir(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<()> {
    static METADATA_CACHE: LazyLock<DashMap<PathBuf, Option<fs::Metadata>>> =
        LazyLock::new(DashMap::new);

    fn get_metadata(path: impl AsRef<Path>) -> Option<fs::Metadata> {
        let path = path.as_ref();

        if let Some(entry) = METADATA_CACHE.get(path) {
            return entry.value().clone();
        }

        let metadata = path.metadata().ok();
        METADATA_CACHE.insert(path.to_owned(), metadata.clone());
        metadata
    }

    /// Compare two files by size and modified time. Returns true if both are equal.
    fn compare_files(a: &Path, b: &Path) -> Result<bool> {
        if let (Some(a), Some(b)) = (get_metadata(a), get_metadata(b)) {
            return Ok(a.len() == b.len() && a.modified()? == b.modified()?);
        }
        Ok(false)
    }

    fn sync(source: &Path, target: &Path) -> Result<()> {
        if get_metadata(target).is_none() {
            fs::create_dir_all(target)?;
        }
        fs::read_dir(source)?
            .par_bridge()
            .try_for_each(|entry| -> Result<()> {
                let entry = entry?;
                let source = entry.path();
                let target = target.join(entry.file_name());
                if get_metadata(&source).is_some_and(|m| m.is_dir()) {
                    if get_metadata(&target).is_some_and(|m| m.is_file()) {
                        fs::remove_file(&target)?;
                    }
                    return sync(&source, &target);
                }
                if get_metadata(&target).is_some_and(|m| m.is_dir()) {
                    rimraf(&target)?;
                }
                if !compare_files(&source, &target)? {
                    fs::copy(source, target)?;
                }
                Ok(())
            })
    }

    /// Remove files that are not present in the source directory.
    fn cleanup(source: &Path, target: &Path) -> Result<()> {
        fs::read_dir(target)?
            .par_bridge()
            .try_for_each(|entry| -> Result<()> {
                let entry = entry?;
                let source = source.join(entry.file_name());
                let target = entry.path();
                let is_dir = get_metadata(&target).is_some_and(|m| m.is_dir());
                if get_metadata(&source).is_none() {
                    if is_dir {
                        rimraf(target)?;
                    } else {
                        fs::remove_file(&target).with_context(|| {
                            format!(
                                "Failed to remove file\n\
                                 <yellow> >></> Path: {}",
                                target.display(),
                            )
                        })?;
                    }
                } else if is_dir {
                    cleanup(&source, &target)?;
                }
                Ok(())
            })
    }

    let source = source.as_ref();
    let target = target.as_ref();
    if get_metadata(target).is_some_and(|m| m.is_dir()) {
        sync(source, target).with_context(|| {
            format!(
                "Failed to copy directory\n\
                 <yellow> >></> From: {}\n\
                 <yellow> >></> To: {}",
                source.display(),
                target.display(),
            )
        })?;
        cleanup(source, target)?;
    } else {
        copy_dir(source, target)?;
    }
    METADATA_CACHE.clear();
    Ok(())
}
