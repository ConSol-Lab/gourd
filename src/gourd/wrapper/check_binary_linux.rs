#![cfg(target_os = "linux")]

use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use elf::abi;
use elf::endian::AnyEndian;
use elf::ElfBytes;
use gourd_lib::bailc;
use gourd_lib::ctx;
use gourd_lib::file_system::FileOperations;

/// A mapping from architecture string to a ELF `e_machine` field.
const E_MACHINE_MAPPING: for<'a> fn(&'a str) -> u16 = |machine| match machine {
    "x86" => abi::EM_IA_64,
    "x86_64" => abi::EM_X86_64,
    "arm" => abi::EM_ARM,
    "aarch64" => abi::EM_AARCH64,
    "mips" => abi::EM_MIPS,
    "mips64" => abi::EM_MIPS_X,
    "powerpc" => abi::EM_PPC,
    "powerpc64" => abi::EM_PPC64,
    "riscv64" => abi::EM_RISCV,
    "s390x" => abi::EM_S390,
    "sparc64" => abi::EM_SPARC,
    _ => 0,
};

/// Verify the architecture of the binary on linux.
pub fn verify_arch(binary: &PathBuf, expected_arch: &str, fs: &impl FileOperations) -> Result<()> {
    let expected_machine_type = E_MACHINE_MAPPING(expected_arch);
    let elf = fs.read_bytes(binary)?;
    let mut elf_iter = elf.iter();

    // We *DO* allow shebangs.
    if let Some(0x23) = elf_iter.next() {
        if let Some(0x21) = elf_iter.next() {
            return Ok(());
        }
    }

    let elf = ElfBytes::<AnyEndian>::minimal_parse(elf.as_slice()).with_context(ctx!(
      "Could not parse the file as ELF {binary:?}", ;
      "Are you sure this file is executable and you are using linux?",
    ))?;

    if elf.ehdr.e_machine != expected_machine_type {
        bailc!(
          "The program architecture {} does not match the expected architecture {expected_arch}",
          elf.ehdr.e_machine;
          "The architecture does not match for program {binary:?}", ;
          "Ensure that the program is compiled for the correct target",
        )
    } else {
        Ok(())
    }
}
