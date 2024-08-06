#![cfg(target_os = "macos")]

use std::path::PathBuf;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::ctx;
use gourd_lib::file_system::FileOperations;

/// Mapping of the expected architecture to the string used by `lipo`.
const OSX_ARCH_MAPPING: for<'a> fn(&'a str) -> &'static str = |machine| match machine {
    "x86" => "i386",
    "x86_64" => "x86_64",
    "aarch64" => "arm64",
    "powerpc" => "ppc",
    _ => "unsupported",
};

/// Verifies that the file present at `binary` matches the CPU architecture
/// provided, specifically for macOS systems. Uses the command-line utility
/// `lipo` to determine what architecture(s) the binary runs on. If `lipo`
/// cannot be called, such as on Macs running OS X/PowerPC that have not
/// received software updates since 2005, the architecture verification is
/// skipped.
pub(crate) fn verify_arch(
    binary: &PathBuf,
    expected_arch: &str,
    fs: &impl FileOperations,
) -> Result<()> {
    let mut bytes = fs
        .read_bytes(binary)
        .context("Could not read the binary file.")?
        .into_iter();

    // We *DO* allow shebangs.
    if let Some(0x23) = bytes.next() {
        if let Some(0x21) = bytes.next() {
            return Ok(());
        }
    }

    match Command::new("lipo")
        .arg("-archs")
        .arg(binary.as_path().as_os_str())
        .output()
    {
        Ok(out) => {
            let binary_archs =
                String::from_utf8(out.stdout.to_ascii_lowercase()).with_context(ctx!(
                    "Could not get the output of 'lipo' when checking the binary's architecture.", ;
                    "",
                ))?;
            // `lipo` can return multiple architectures (macOS Universal Binary)
            if !binary_archs.contains(OSX_ARCH_MAPPING(expected_arch)) {
                bailc!(
                  "The program architecture(s) {binary_archs} do not match \
                    the expected architecture {expected_arch}", ;
                  "The architecture does not match for program {binary:?}", ;
                  "Ensure that the program is compiled for the correct target",
                );
            }
            Ok(())
        }
        Err(_) => Ok(()),
    }
}
