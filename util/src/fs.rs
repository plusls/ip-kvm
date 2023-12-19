use std::fs;
use std::fs::ReadDir;
use std::num::ParseIntError;
use std::path::Path;

use crate::error;

pub fn read<P: AsRef<Path>>(path: P) -> error::Result<Vec<u8>> {
    Ok(fs::read(&path).map_err(|err| error::ErrorKind::io(err, path))?)
}

pub fn read_to_string<P: AsRef<Path>>(path: P) -> error::Result<String> {
    Ok(fs::read_to_string(&path).map_err(|err| error::ErrorKind::io(err, path))?)
}

pub fn read_to_num<P: AsRef<Path>, R: num_traits::Num<FromStrRadixErr = ParseIntError>>(
    path: P,
) -> error::Result<R> {
    let s = read_to_string(path)?;
    Ok(parse_int::parse(&s).map_err(error::DeserializedError::from)?)
}

pub fn read_to_bool<P: AsRef<Path>>(path: P) -> error::Result<bool> {
    let tmp_s = read_to_string(path)?;
    let s = tmp_s.trim();
    Ok(match s {
        "1" => true,
        "0" => false,
        "true" => true,
        "false" => false,
        _ => Err(error::DeserializedError::Custom(format!(
            "Can not parse {s} to boolean."
        )))?,
    })
}

pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> error::Result<()> {
    Ok(fs::write(&path, contents).map_err(|err| error::ErrorKind::io(err, path))?)
}

pub fn remove_dir<P: AsRef<Path>>(path: P) -> error::Result<()> {
    Ok(fs::remove_dir(&path).map_err(|err| error::ErrorKind::io(err, path))?)
}

pub fn remove_file<P: AsRef<Path>>(path: P) -> error::Result<()> {
    Ok(fs::remove_file(&path).map_err(|err| error::ErrorKind::io(err, path))?)
}

pub fn read_dir<P: AsRef<Path>>(path: P) -> error::Result<ReadDir> {
    Ok(fs::read_dir(&path).map_err(|err| error::ErrorKind::io(err, path))?)
}

pub fn create_dir<P: AsRef<Path>>(path: P) -> error::Result<()> {
    Ok(fs::create_dir(&path).map_err(|err| error::ErrorKind::io(err, path))?)
}

pub fn symlink_metadata<P: AsRef<Path>>(path: P) -> error::Result<fs::Metadata> {
    Ok(fs::symlink_metadata(&path).map_err(|err| error::ErrorKind::io(err, path))?)
}
