//!
//! # HFS path conversion #
//!
//! Some Mac OS APIs use HFS paths, which use `:` as a directory separator and start with a
//! volume name. This crate provides a function to convert them into standard paths that start
//! with `/` and use `/` as a directory separator.
//!

#[macro_use]
extern crate quick_error;

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Converts the provided HFS path into a standard POSIX path
pub fn convert_path(path: &str) -> Result<PathBuf> {
    // : is the directory separator
    let mut segments = path.split(':');
    // Check for the volume name as the first path segment
    match segments.next() {
        Some(volume_name) => {
            // Replace slashes with colons
            let volume_name = volume_name.replace('/', ":");
            // Find the POSIX path to this volume
            let mut path = find_volume(&volume_name)?;
            // Append other path segments and separators
            for segment in segments {
                let segment = segment.replace('/', ":");
                path = path.join(segment);
            }
            Ok(path)
        }
        None => Err(Error::InvalidHfsPath)
    }
}

/// Looks for a volume with the provided name and returns the absolute path to its root
fn find_volume(name: &str) -> Result<PathBuf> {
    for entry in fs::read_dir("/Volumes")? {
        let entry = entry?;
        if entry.file_name() == OsStr::new(name) {
            if entry.file_type()?.is_symlink() {
                // Follow link
                let link_dest = fs::read_link(entry.path())?;
                return Ok(link_dest)
            } else {
                return Ok(entry.path())
            }
        }
    }
    Err(Error::VolumeNotFound(name.into()))
}

pub type Result<T> = ::std::result::Result<T, Error>;

quick_error! {
    /// Conversion errors
    #[derive(Debug)]
    pub enum Error {
        /// An HFS path with an invalid format was provided
        InvalidHfsPath {
            description("invalid HFS path format")
            display("Invalid HFS path format")
        }
        /// A mounted volume with the specified name was not found
        VolumeNotFound(volume: String) {
            description("volume not found")
            display("Volume {} not found", volume)
        }
        /// An IO error occurred
        Io(err: io::Error) {
            description("I/O error")
            cause(err)
            from()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! expect {
        { $( $hfs:expr => $expected:expr ),+ } => {
            [ $( ($hfs, $expected) ),* ]
        };
        { } => { [("", ""); 0] };
    }

    #[test]
    fn test_paths() {
        // Note: These tests depend on the layout of volumes on the computer that they run on.
        // The volumes must be present for the tests to pass.
        let tests = expect! {
            "Macintosh SSD:folder1:file" => "/folder1/file",
            "Macintosh SSD" => "/",
            "Macintosh SSD:folder/with/slashes:file.txt" => "/folder:with:slashes/file.txt",
            "BOOTCAMP:Intel:Logs:IntelGFX.log" => "/Volumes/BOOTCAMP/Intel/Logs/IntelGFX.log"
        };

        for &(hfs, expected) in tests.into_iter() {
            let actual = convert_path(hfs).unwrap();
            assert_eq!(expected, &actual.display().to_string());
        }
    }
}
