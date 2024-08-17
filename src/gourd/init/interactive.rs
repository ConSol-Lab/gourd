use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::config::slurm::SlurmConfig;
use gourd_lib::config::Config;
use gourd_lib::constants::CMD_STYLE;
use gourd_lib::ctx;
use gourd_lib::file_system::FileOperations;
use inquire::error::InquireResult;
use inquire::validator::ValueRequiredValidator;
use inquire::Confirm;
use inquire::InquireError;
use inquire::Text;
use log::debug;
use log::info;

/// Correctly handles when the user cancels the operation
/// during an Inquire prompt.
#[cfg(not(tarpaulin_include))]
pub fn ask<T>(inq: InquireResult<T>) -> Result<T> {
    match inq {
        Ok(answer) => Ok(answer),
        Err(InquireError::OperationCanceled) => bailc!(
                    "The operation was canceled using the ESC key.", ; "",; "",),
        Err(InquireError::OperationInterrupted) => {
            bailc!("The operation was interrupted using Ctrl+C.", ; "",; "",)
        }
        other => other.with_context(ctx!("Could not print a prompt", ; "", )),
    }
}

/// Initializes an experimental setup using interactive prompts.
pub fn init_interactive(
    directory: &Path,
    script_mode: bool,
    fs: &impl FileOperations,
) -> Result<()> {
    if !script_mode {
        info!("");
        info!("The following questions will help you customize a `gourd.toml` experimental setup.");
        info!("Use {CMD_STYLE}gourd init -s{CMD_STYLE:#} to skip customization.");
        info!("");
    }

    let mut config = Config {
        output_path: PathBuf::from("experiments"),
        metrics_path: PathBuf::from("experiments"),
        experiments_folder: PathBuf::from("experiments"),
        programs: Default::default(),
        inputs: Default::default(),
        parameters: None,
        slurm: None,
        resource_limits: None,
        wrapper: Default::default(),
        labels: None,
        input_schema: None,
        warn_on_label_overlap: false,
    };

    let custom_paths = if script_mode {
        false
    } else {
        ask(Confirm::new("Specify custom output paths?")
            .with_help_message("By default, metrics and outputs go in the 'experiments' directory.")
            .prompt())?
    };

    if custom_paths {
        config.experiments_folder = PathBuf::from(ask(Text::new("Path to experiments folder: ")
            .with_default(config.experiments_folder.to_str().unwrap())
            .with_validator(ValueRequiredValidator::default())
            .with_help_message("A folder where Gourd will store runtime data.")
            .prompt())?);

        config.output_path = PathBuf::from(ask(Text::new("Path to output folder: ")
            .with_default(config.output_path.to_str().unwrap())
            .with_validator(ValueRequiredValidator::default())
            .with_help_message("A folder where programs' outputs will be organised.")
            .prompt())?);

        config.metrics_path = PathBuf::from(ask(Text::new("Path to metrics folder: ")
            .with_default(config.metrics_path.to_str().unwrap())
            .with_validator(ValueRequiredValidator::default())
            .with_help_message("A folder where metrics will be organised.")
            .prompt())?);
    }

    let slurm = if script_mode {
        true
    } else {
        ask(Confirm::new("Include options for Slurm?")
            .with_help_message("These will allow the experiment to run on a cluster computer.")
            .prompt())?
    };

    if slurm {
        // These defaults are used in script mode and for user input.
        let mut slurm_config = SlurmConfig {
            experiment_name: "my-experiment".to_string(),
            output_folder: Default::default(), // todo: entered by user
            partition: "".to_string(),
            array_size_limit: None,
            max_submit: None,
            account: "".to_string(),
            begin: None,
            mail_type: None,
            mail_user: None,
            additional_args: None,
        };

        if !script_mode {
            slurm_config.experiment_name = ask(Text::new("Slurm experiment name: ")
                .with_validator(ValueRequiredValidator::default())
                .with_help_message("This will be used to name jobs submitted to Slurm.")
                .prompt())?;

            let enter_slurm_data_now = ask(Confirm::new("Enter Slurm credentials now?")
                .with_help_message(
                    "Choosing 'no' will leave the 'account' and 'partition' blank for now.",
                )
                .prompt())?;

            if enter_slurm_data_now {
                slurm_config.account = ask(Text::new("Slurm account to use: ")
                    .with_validator(ValueRequiredValidator::default())
                    .with_help_message(
                        "This should be provided by the administrator of your supercomputer.",
                    )
                    .prompt())?;

                slurm_config.partition = ask(Text::new("Slurm partition to use: ")
                    .with_help_message("Most supercomputers use this to choose types of nodes.")
                    .prompt())?;
            }
        }

        config.slurm = Some(slurm_config);
    }

    write_files(directory, config, fs)
}

/// Write all files during initialization.
pub fn write_files(directory: &Path, config: Config, fs: &impl FileOperations) -> Result<()> {
    if directory.exists() {
        bailc!(
            "The path exists.", ;
            "A directory or file exists at {directory:?}.", ;
            "Choose a path that is not already taken.",
        );
    }

    let canonical_directory = fs.truncate_and_canonicalize_folder(directory)?;

    let mut toml_path = canonical_directory.to_path_buf();
    toml_path.push("gourd.toml");

    debug!("Creating `gourd.toml` file at {:?}.", &toml_path);
    fs.try_write_toml(&toml_path, &config)?;

    let dirs_to_create = vec![
        Some(config.output_path),
        Some(config.metrics_path),
        Some(config.experiments_folder),
    ];

    debug!("Creating experiment folders.");
    for dir in dirs_to_create.into_iter().flatten() {
        if dir.is_relative() {
            let mut fs_dir_path = canonical_directory.to_path_buf();
            fs_dir_path.push(dir);

            fs.truncate_and_canonicalize_folder(&fs_dir_path)?;
        } else {
            debug!("Skipping non-relative path {:?}", &dir)
        }
    }

    Ok(())
}
