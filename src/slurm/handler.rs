use anyhow::anyhow;
use anyhow::Context;

use crate::cli::printing::format_table;
use crate::config::Config;
use crate::ctx;
use crate::error::Ctx;
use crate::slurm::SlurmConfig;
use crate::slurm::SlurmInteractor;

/// Check the config that it has the necessary fields
pub fn check_config(config: &Config) -> anyhow::Result<&SlurmConfig> {
    config.slurm.as_ref()
        .ok_or_else(|| anyhow!("No SLURM configuration found"))
        .with_context(ctx!(
              "Tried to execute on Slurm but the configuration field for the Slurm options in gourd.toml was empty", ;
              "Make sure that your gourd.toml includes the required fields under [slurm]",
            ))
}

/// Check if the SLURM version is supported.
pub(crate) fn check_version<T>(internal: &T) -> anyhow::Result<()>
where
    T: SlurmInteractor,
{
    match internal.get_version() {
        Ok(version) => {
            if !internal.is_version_supported(version) {
                Err(anyhow!("SLURM Version assertion failed")).with_context(
                    ctx!("Unsupported SLURM version: {:?}",
                      version.iter().map(u64::to_string).collect::<Vec<String>>().join(".");
                      "Supported versions are: {}",
                      internal.get_supported_versions()
                    ),
                )
            } else {
                Ok(())
            }
        }

        Err(e) => Err(anyhow!("SLURM versioning failed")).with_context(ctx!(
          "Failed to get SLURM version: {}", e;
          "Please make sure that SLURM is installed and available in the PATH",
        )),
    }
}

/// Check if the provided partition is valid.
pub fn check_partition<T>(internal: &T, partition: &str) -> anyhow::Result<()>
where
    T: SlurmInteractor,
{
    let partitions = internal.get_partitions()?;
    if partitions.iter().map(|x| x.first()).any(|x| {
        if let Some(y) = x {
            y == partition
        } else {
            false
        }
    }) {
        Ok(())
    } else {
        Err(anyhow!("Invalid partition provided")).with_context(ctx!(
          "Partition `{:?}` is not available on this cluster. ", partition;
          "Present partitions are:\n{:?}", format_table(partitions),
        ))
    }
}
