use std::fs::{self, File};
use std::io::{Error, ErrorKind, Read, Write};

pub fn mkdir(dirpath: String) -> Result<(), std::io::Error> {
    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => {
            let dirpath = format!("{}/.hermit/{}", path.display(), dirpath);
            match fs::create_dir_all(&dirpath) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        },
        _ => {
            return Err(Error::new(ErrorKind::NotFound, "Home directory not found."));
        },
    }
}

pub fn mkfile(filepath: String) -> Result<(), std::io::Error> {
    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => {
            let filepath = format!("{}/.hermit/{}", path.display(), filepath);
            match fs::File::create(filepath) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        },
        _ => {
            return Err(Error::new(ErrorKind::NotFound, "Home directory not found."));
        },
    }
}

pub fn read_file(filepath: String) -> Result<Vec<u8>, Error> {
    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => {
            let filepath = format!("{}/.hermit/{}", path.display(), filepath);
            let mut f = File::open(filepath)?;
            let mut data = vec![];
            f.read_to_end(&mut data)?;

            Ok(data)
        },
        _ => Err(Error::new(ErrorKind::NotFound, "Home directory not found.")),
    }
}

pub fn write_file(filepath: String, data: &[u8]) -> Result<(), Error> {
    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => {
            let filepath = format!("{}/.hermit/{}", path.display(), filepath);
            let mut f = File::create(filepath)?;
            f.write_all(data)?;

            Ok(())
        },
        _ => Err(Error::new(ErrorKind::NotFound, "Home directory not found.")),
    }
}

pub fn empty_file(filepath: String) -> Result<(), Error> {
    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => {
            let filepath = format!("{}/.hermit/{}", path.display(), filepath);
            // OpenOptions::new().truncate(true).open(filepath)?;
            let mut f = File::create(filepath)?;
            f.write_all(b"")?;
            Ok(())
        },
        _ => Err(Error::new(ErrorKind::NotFound, "Home directory not found.")),
    }
}