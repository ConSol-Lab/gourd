#![cfg(not(tarpaulin_include))]

use std::fmt::Debug;
use std::path::PathBuf;

use elf::to_str::e_machine_to_human_str;
use elf::ParseError;

/// This error type is used by all gourd functions.
pub enum GourdError {
    /// The architecture does not match the one we want to run on.
    ArchitectureMismatch {
        /// The expected architecture in `e_machine` format.
        expected: u16,

        /// The architecture of the binary in `e_machine` format.
        binary: u16,
    },

    /// A filesystem error occured.
    FileError(PathBuf, std::io::Error),

    /// A file unrelated filesystem error occured.
    IoError(std::io::Error),

    /// This ELF file failed to parse
    ElfParseError(ParseError),
}

impl Debug for GourdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArchitectureMismatch { expected, binary } => write!(
                f,
                "The {:?} architecture does not match {:?}, the runners architecture",
                e_machine_to_human_str(*binary),
                e_machine_to_human_str(*expected)
            ),
            Self::FileError(file, io_err) => {
                write!(f, "Could not access file {:?}: {}", file, io_err)
            }
            Self::IoError(io_err) => write!(f, "A IO error occured: {}", io_err),
            Self::ElfParseError(err) => write!(f, "This is not a valid elf file: {}", err),
        }
    }
}

impl From<std::io::Error> for GourdError {
    fn from(value: std::io::Error) -> Self {
        GourdError::IoError(value)
    }
}

impl From<ParseError> for GourdError {
    fn from(value: ParseError) -> Self {
        GourdError::ElfParseError(value)
    }
}
