use std::fs::File;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;

use crate::ctx;
use crate::error::Ctx;
use crate::file_system::FileOperations;

/// Gets the files given the filepaths.
#[allow(unused)]
pub fn get_resources(filepaths: Vec<&Path>) -> Result<Vec<File>> {
    let mut files: Vec<File> = vec![];

    for path in filepaths {
        files.push(File::open(path).with_context(ctx!(
          "Could not open resource file {path:?}", ;
          "Ensure that the file exists",
        ))?);
    }

    Ok(files)
}

/// Downloads a file given a url.
pub fn download_exec(url: &str, output_path: &Path, fs: &impl FileOperations) -> Result<()> {
    let response = ureq::get(url).call().with_context(ctx!(
      "Could not access the resource at {url}", ;
      "Check that the url is correct",
    ))?;
    let mut body: Vec<u8> = Vec::new();

    response
        .into_reader()
        .read_to_end(&mut body)
        .with_context(ctx!(
            "Could not parse the resource at {url}", ;
            "Check that the url is not misspelled",
        ))?;

    fs.write_bytes_truncate(output_path, &body)?;
    fs.make_executable(output_path)?;

    Ok(())
}

#[cfg(test)]
#[path = "tests/network.rs"]
mod tests;
