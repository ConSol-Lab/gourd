#![cfg(feature = "builtin-examples")]

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use flate2::read::GzDecoder;
use gourd_lib::config::Config;
use gourd_lib::file_system::FileOperations;
use gourd_lib::file_system::FileSystemInteractor;
use log::debug;
use log::warn;
use tar::Archive;

/// Stores a template for `gourd init`: a named set of directory contents.
#[derive(Debug, Clone)]
pub struct InitExample<'a> {
    /// The template name.
    pub name: &'a str,

    /// The template description.
    pub description: &'a str,

    /// A tar-gz encoded version of the directory structure
    /// of the given template/example, containing all files
    /// within the example directory excluding `gourd.toml`.
    pub directory_tarball: &'a [u8],
}

impl InitExample<'_> {
    /// Extracts the template's tarball in the provided directory.
    ///
    /// The directory must have a valid parent, but may not exist.
    /// This is to be enforced by the caller method.
    pub fn unpack_to(&self, directory: &Path, file_system: &FileSystemInteractor) -> Result<()> {
        let tar = GzDecoder::new(self.directory_tarball);
        let mut archive = Archive::new(tar);

        // Do not preserve the creation time, etc. when unpacking the template.
        archive.set_preserve_mtime(false);

        debug!("Unpacking the example archive");
        file_system.write_archive(directory, archive)?;

        if !file_system.dry_run {
            debug!("Entering the directory {directory:?}");
            let previous_dir = std::env::current_dir()?;
            std::env::set_current_dir(directory)?;

            let config_path = PathBuf::from("gourd.toml");

            debug!("Checking for a \"gourd.toml\" at {config_path:?}.");
            match Config::from_file(Path::new("gourd.toml"), file_system) {
                Err(e) => {
                    debug!("Configuration check failed: {}", e.root_cause());
                    warn!(
                        "The \"gourd.toml\" configuration in this example is missing or invalid."
                    );
                    warn!("You may have to make some changes.");
                }
                Ok(_) => debug!("A valid \"gourd.toml\" is present."),
            }

            debug!("Returning to the previous directory {:?}", &previous_dir);
            std::env::set_current_dir(previous_dir)?;
        }

        Ok(())
    }
}

/// Retrieves all available examples.
pub fn get_examples() -> BTreeMap<&'static str, InitExample<'static>> {
    let mut examples = BTreeMap::new();

    examples.insert(
        "fibonacci-comparison",
        InitExample {
            name: "Fibonacci Comparison",
            description: "A simple intro to designing Gourd experiments with programs and inputs.",

            directory_tarball: include_bytes!(concat!(
                env!("OUT_DIR"),
                "/../../../tarballs/fibonacci-comparison.tar.gz"
            )),
        },
    );
    examples.insert(
        "grid-search",
        InitExample {
            name: "Grid Search",
            description: "An example of exhaustive grid search with parameters",

            directory_tarball: include_bytes!(concat!(
                env!("OUT_DIR"),
                "/../../../tarballs/grid-search.tar.gz"
            )),
        },
    );

    examples
}

/// Retrieves a named example experiment, or [None].
pub fn get_example(id_input: &str) -> Option<InitExample<'static>> {
    let id = id_input.to_string().replace(['.', '_', ' '], "-");
    debug!("Translating the example-id: {id_input} to {id}");
    get_examples().get(&id as &str).cloned()
}
