use std::fs;
use std::fs::ReadDir;
use std::path::Path;

use crate::error;

pub fn read<P: AsRef<Path>>(path: P) -> error::Result<Vec<u8>> {
    Ok(fs::read(&path).map_err(|err| error::ErrorKind::fs(err, path))?)
}

pub fn read_to_string<P: AsRef<Path>>(path: P) -> error::Result<String> {
    Ok(fs::read_to_string(&path).map_err(|err| error::ErrorKind::fs(err, path))?)
}

pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> error::Result<()> {
    Ok(fs::write(&path, contents).map_err(|err| error::ErrorKind::fs(err, path))?)
}

pub fn remove_dir<P: AsRef<Path>>(path: P) -> error::Result<()> {
    Ok(fs::remove_dir(&path).map_err(|err| error::ErrorKind::fs(err, path))?)
}

pub fn remove_file<P: AsRef<Path>>(path: P) -> error::Result<()> {
    Ok(fs::remove_file(&path).map_err(|err| error::ErrorKind::fs(err, path))?)
}

pub fn read_dir<P: AsRef<Path>>(path: P) -> error::Result<ReadDir> {
    Ok(fs::read_dir(&path).map_err(|err| error::ErrorKind::fs(err, path))?)
}

pub fn create_dir<P: AsRef<Path>>(path: P) -> error::Result<()> {
    Ok(fs::create_dir(&path).map_err(|err| error::ErrorKind::fs(err, path))?)
}

pub fn symlink_metadata<P: AsRef<Path>>(path: P) -> error::Result<fs::Metadata> {
    Ok(fs::symlink_metadata(&path).map_err(|err| error::ErrorKind::fs(err, path))?)
}