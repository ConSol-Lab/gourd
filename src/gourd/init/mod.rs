/// Interactive configuration of an experiment setup template.
pub mod interactive;

/// Tarballs of built-in example templates (optional feature)
pub mod builtin_examples;

use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::constants::CMD_STYLE;
use gourd_lib::ctx;
use gourd_lib::file_system::FileOperations;
use gourd_lib::file_system::FileSystemInteractor;
use log::debug;
use log::info;

use crate::init::interactive::init_interactive;

/// Lists valid example IDs for this build of `gourd init`.
///
/// Returns the ID, name, and description for each example identifier that
/// `gourd init -e <example-id>` can be called on. If the optional feature
/// for `builtin-examples` has not been compiled, shows an error message
/// that indicates this.
pub fn list_init_examples() -> Result<()> {
    #[cfg(not(feature = "builtin-examples"))]
    bailc!(
        "No examples are available.", ;
        "This version of gourd was not compiled with built-in examples.", ;
        "To include these, build with the \"builtin-examples\" feature.",
    );

    #[cfg(feature = "builtin-examples")]
    {
        use crate::init::builtin_examples::get_examples;

        for example in get_examples() {
            println!("\"{}\" - {}", example.0, example.1.name);
            println!("    {}", example.1.description)
        }

        Ok(())
    }
}

/// Initializes an experimental setup.
///
/// This method performs `gourd init` at the provided directory, first
/// verifying that the directory does NOT exist but has a valid parent.
/// If a valid experiment ID is present, the pre-compiled tarball for
/// that experiment is unpacked. Otherwise, the setup is generated
/// interactively. The `script_mode` parameter only takes effect with
/// no template and forces defaults for all values. Finally, a Git
/// repository is initialised if all the previous steps succeed and
/// `use_git` is true.
pub fn init_experiment_setup(
    directory: &Path,
    use_git: bool,
    script_mode: bool,
    dry_run: bool,
    template: &Option<String>,
    fs: &FileSystemInteractor,
) -> Result<()> {
    check_init_directory(directory)?;

    info!("Creating an experimental setup at {directory:?}." );

    match template {
        None => init_interactive(directory, script_mode, fs)?,
        Some(t) => init_from_example(t, directory, fs)?,
    }

    if use_git {
        debug!("Initialising a Git repository.");

        fs.init_git_repository(directory).with_context(
            ctx!("Error creating a Git repository, the experimental setup was still created.", ;
                "Use the '--git=false' option to skip Git." ,),
        )?;
    }

    if dry_run {
        info!("The experimental setup would be ready in {directory:?} (dry)");
    } else {
        info!("");
        info!("The experimental setup is ready in {directory:?}!");
        info!("To create an experiment, use these commands:");
        info!(" >  {CMD_STYLE}cd {directory:?}{CMD_STYLE:#}");
        info!(" >  {CMD_STYLE}gourd run{CMD_STYLE:#}");
    }

    Ok(())
}

/// Verifies that a directory is valid for the `gourd init` command.
///
/// This entails checking that the parent of the provided path is
/// valid (even for relative paths such as "foo" that have "" as parent),
/// and checking that the directory path itself does not yet exist.
fn check_init_directory(directory: &Path) -> Result<()> {
    debug!("Checking the init directory at {directory:?}");

    match directory.parent() {
        // The directory is "/", the next check will fail (already exists)
        None => {}
        // Check that the parent exists
        Some(parent) => {
            debug!("Checking the parent directory at {parent:?}");

            if !(parent.exists() || parent == Path::new("")) {
                bailc!(
                  "The parent directory, {parent:?}, does not exist.", ;
                  "", ;
                  "Ensure that you are asking 'gourd init' to \
                  initialize the directory at a valid path.",
                )
            }

            debug!("The parent directory, {parent:?}, is valid",)
        }
    }

    if directory.exists() {
        bailc!(
          "The path exists.", ;
          "A file or directory exists at {directory:?}.", ;
          "Run 'gourd init' for a directory that does not yet exist.",
        )
    }

    Ok(())
}

/// Retrieves and unpacks the example denoted by the provided example-id.
fn init_from_example(
    #[allow(unused)] id: &str,
    #[allow(unused)] directory: &Path,
    #[allow(unused)] fs: &FileSystemInteractor,
) -> Result<()> {
    #[cfg(not(feature = "builtin-examples"))]
    bailc!(
        "Cannot use the -e flag.", ;
        "This version of gourd was not compiled with built-in examples.", ;
        "To include these, build with the \"builtin-examples\" feature.",
    );

    #[cfg(feature = "builtin-examples")]
    {
        use crate::init::builtin_examples::get_example;
        use crate::init::builtin_examples::get_examples;

        match get_example(id) {
            None => bailc!(
                "Invalid example name.", ;
                "An example called \"{id}\" does not exist.",  ;
                "Try a valid example, such as \"{}\". \
                Use {CMD_STYLE}gourd init --list-examples{CMD_STYLE:#} for all options.",
                get_examples().iter().next().unwrap().0,
            ),
            Some(example) => {
                info!("Selected example: {}", example.name);
                info!("   {}", example.description);
                info!("");
                example.unpack_to(directory, fs)
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
