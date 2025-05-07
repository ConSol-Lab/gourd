use std::env;
use std::io::stdout;
use std::process::exit;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use chrono::Local;
use clap::CommandFactory;
use clap::FromArgMatches;
use colog::default_builder;
use colog::formatter;
use gourd_lib::bailc;
use gourd_lib::config::Config;
use gourd_lib::constants::CMD_DOC_STYLE;
use gourd_lib::constants::CMD_STYLE;
use gourd_lib::constants::ERROR_STYLE;
use gourd_lib::constants::PATH_STYLE;
use gourd_lib::constants::PRIMARY_STYLE;
use gourd_lib::constants::TERTIARY_STYLE;
use gourd_lib::ctx;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use gourd_lib::file_system::FileSystemInteractor;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use log::debug;
use log::info;
use log::trace;
use log::LevelFilter;

use super::def::ContinueStruct;
use super::def::RerunOptions;
use super::log::LogTokens;
use super::printing::get_styles;
use crate::analyse::csvs::tables_from_command;
use crate::analyse::plotting::analysis_plot;
use crate::chunks::Chunkable;
use crate::cli::def::AnalSubcommand;
use crate::cli::def::AnalyseStruct;
use crate::cli::def::CancelStruct;
use crate::cli::def::Cli;
use crate::cli::def::GourdCommand;
use crate::cli::def::RunSubcommand;
use crate::cli::def::StatusStruct;
use crate::cli::printing::print_version;
use crate::experiments::run::generate_new_run;
use crate::experiments::ExperimentExt;
use crate::init::init_experiment_setup;
use crate::init::list_init_examples;
use crate::local::run_local;
use crate::post::afterscript::run_afterscripts_for_experiment;
use crate::rerun;
use crate::rerun::slurm::query_changing_resource_limits;
use crate::slurm::checks::slurm_options_from_experiment;
use crate::slurm::handler::SlurmHandler;
use crate::slurm::interactor::SlurmCli;
use crate::slurm::SlurmInteractor;
use crate::status::blocking_status;
use crate::status::chunks::print_scheduling;
use crate::status::printing::display_job;
use crate::status::printing::display_statuses;
use crate::status::DynamicStatus;

/// This function parses command that gourd was run with.
pub async fn parse_command() {
    let styled = Cli::command().styles(get_styles()).get_matches();

    // This unwrap will print the error if the command is wrong.
    let command = Cli::from_arg_matches(&styled).unwrap();

    // https://github.com/rust-lang/rust/blob/master/library/std/src/backtrace.rs
    let backtrace_enabled = match env::var("RUST_LIB_BACKTRACE") {
        Ok(s) => s != "0",
        Err(_) => match env::var("RUST_BACKTRACE") {
            Ok(s) => s != "0",
            Err(_) => false,
        },
    };

    if backtrace_enabled {
        eprintln!("{:?}", process_command(&command).await);
    } else if let Err(e) = process_command(&command).await {
        eprintln!("{ERROR_STYLE}error:{ERROR_STYLE:#} {}", e.root_cause());
        eprint!("{e}");
        exit(1);
    }
}

/// CLAP has parsed the command, now we process it.
pub async fn process_command(cmd: &Cli) -> Result<()> {
    let progress = setup_logging(cmd)?;

    let mut file_system = FileSystemInteractor { dry_run: cmd.dry };

    /// Get the experiment instance from the config file and the experiment
    /// folder.
    fn read_experiment(
        experiment_id: &Option<usize>,
        cmd: &Cli,
        file_system: &FileSystemInteractor,
    ) -> Result<Experiment> {
        debug!("Reading the config: {:?}", cmd.config);

        let conf: Config = file_system.try_read_toml(&cmd.config)?;
        let exp = if let Some(id) = experiment_id {
            Experiment::experiment_from_folder(*id, &conf.experiments_folder, file_system)?
        } else {
            Experiment::latest_experiment_from_folder(&conf.experiments_folder, file_system)?
        };

        debug!("Found the newest experiment with id: {}", exp.seq);

        Ok(exp)
    }

    match &cmd.command {
        GourdCommand::Run(args) => {
            debug!("Reading the config: {:?}", cmd.config);

            let config = Config::from_file(&cmd.config, &file_system)?;

            debug!("Creating a new experiment");
            trace!("The config is: {config:#?}");

            let mut experiment = Experiment::from_config(
                &config,
                Local::now(),
                match args.subcommand {
                    RunSubcommand::Local { .. } => Environment::Local,
                    RunSubcommand::Slurm { .. } => Environment::Slurm,
                },
                &file_system,
            )?;

            let exp_path = experiment.save(&file_system)?;
            debug!("Saved the experiment at {exp_path:?}");

            match args.subcommand {
                RunSubcommand::Local { force, sequential } => {
                    if cmd.dry {
                        info!("Would have ran the experiment (dry)");
                    } else {
                        let to_complete =
                            run_local(&mut experiment, &file_system, force, sequential).await?;

                        info!("Experiment started");

                        // Local will never unshorten status, hence the false.
                        blocking_status(
                            &progress,
                            &experiment,
                            &mut file_system,
                            false,
                            to_complete,
                        )?;

                        info!("Experiment finished");
                        println!();
                    }
                }

                RunSubcommand::Slurm { .. } => {
                    let s: SlurmHandler<SlurmCli> = SlurmHandler::default();
                    s.check_version()?;
                    s.check_partition(&slurm_options_from_experiment(&experiment)?.partition)?;

                    if cmd.dry {
                        info!("Would have scheduled the experiment on slurm (dry)");
                    } else {
                        s.run_experiment(&mut experiment, exp_path.clone(), &file_system)?;
                        print_scheduling(&experiment, true)?;
                        info!("Experiment started");
                    }

                    experiment.save(&file_system)?;
                }
            }

            if cmd.dry {
                info!(
                    "This was a dry run, {PRIMARY_STYLE}gourd status {}{PRIMARY_STYLE:#} \
                    will not display anything",
                    experiment.seq
                );
            } else {
                info!(
                    "Run {PRIMARY_STYLE} gourd status {} {PRIMARY_STYLE:#} to check on this experiment",
                    experiment.seq
                );
                info!(
                    " or {PRIMARY_STYLE} gourd status {} -i <run number> {PRIMARY_STYLE:#}\
                    to check on a specific run",
                    experiment.seq
                );
            }
            if cmd.script {
                println!("{}", exp_path.display());
            }
        }

        GourdCommand::Status(StatusStruct {
            experiment_id,
            run_id,
            follow: blocking,
            full,
            after_out,
            ..
        }) => {
            let mut experiment = read_experiment(experiment_id, cmd, &file_system)?;

            // first run the afterscripts:
            run_afterscripts_for_experiment(&mut experiment, &file_system)?;
            // then get the statuses
            let statuses = experiment.status(&file_system)?;

            match run_id {
                Some(id) => {
                    // only print the afterscript output,
                    // in a script-readable format (nothing fancy)
                    if *after_out {
                        if let Some(Some(out)) = &experiment
                            .runs
                            .get(*id)
                            .map(|x| x.afterscript_output.clone())
                        {
                            println!("{out}");
                        } else {
                            // INFO: here we can detect if the run is supposed to have an
                            // afterscript ourselves. but for now the user can do it instead.
                            bailc!(
                                "could not print afterscript output",;
                                "there is no afterscript output for this run",;
                                "please make sure
1. this run is supposed to have an afterscript, and
2. that the afterscript prints its output correctly.",
                            );
                        }
                    } else {
                        display_job(&mut stdout(), &experiment, &statuses, *id)?;
                    }
                }
                None => {
                    info!(
                        "Displaying the status of jobs for experiment {}",
                        experiment.seq
                    );

                    let run_count = experiment.runs.len();

                    if *blocking {
                        blocking_status(
                            &progress,
                            &experiment,
                            &mut file_system,
                            *full,
                            run_count,
                        )?;
                    } else {
                        display_statuses(&mut stdout(), &experiment, &statuses, *full)?;
                    }
                }
            }
        }

        GourdCommand::Init(init_struct) => {
            if init_struct.list_examples {
                list_init_examples()?;
            } else {
                match &init_struct.directory {
                    None => bailc!("No directory specified", ;
                      "", ;
                          "You need to specify a directory for init\
                        , for example: {CMD_STYLE}gourd init test{CMD_STYLE:#}",
                    ),

                    Some(directory) => init_experiment_setup(
                        directory,
                        init_struct.git,
                        cmd.script,
                        cmd.dry,
                        &init_struct.example,
                        &file_system,
                    )?,
                }
            }
        }

        // handle plotting separately
        GourdCommand::Analyse(AnalyseStruct {
            experiment_id,
            subcommand:
                AnalSubcommand::Plot {
                    format,
                    output: save_a,
                },
            output: save_b,
        }) => {
            let experiment = read_experiment(experiment_id, cmd, &file_system)?;

            let statuses = experiment.status(&file_system)?;

            let out_path =
                save_a
                    .clone()
                    .or(save_b.clone())
                    .unwrap_or(experiment.home.join(format!(
                        "plot_{}.{}",
                        experiment.seq,
                        format.ext()
                    )));

            if statuses
                .values()
                .filter(|x| x.fs_status.completion.is_completed())
                .count()
                == 0
            {
                bailc!(
                    "No runs have completed yet", ;
                    "There are no results to analyse.", ;
                    "Try again later. To see job status, use {CMD_STYLE}gourd status{CMD_STYLE:#}.",
                );
            }

            if cmd.dry {
                return Ok(());
            } else {
                let out = analysis_plot(&out_path, statuses, &experiment, *format)?;
                info!("Plot saved to:");
                println!("{PATH_STYLE}{}{PATH_STYLE:#}", out.display());
                // non-info printing can let scripts easily get the path from
                // the last line.
            }
        }

        GourdCommand::Analyse(AnalyseStruct {
            experiment_id,
            subcommand: AnalSubcommand::Table(csv),
            output: save,
        }) => {
            let experiment = read_experiment(experiment_id, cmd, &file_system)?;
            let statuses = experiment.status(&file_system)?;

            let tables = tables_from_command(&experiment, &statuses, csv.clone())?;

            if let Some(path) = &save.clone().or(csv.output.clone()) {
                let count = tables.len();
                if cmd.dry {
                    info!("Would have saved {count} table(s) to {}", path.display());
                } else {
                    for table in tables {
                        table.write_to_path(path)?;
                    }
                    info!("{count} Table{} saved to", if count > 1 { "s" } else { "" });
                    println!("{PATH_STYLE}{}{PATH_STYLE:#}", path.display());
                    // non-info printing can let scripts easily get the path
                    // from the last line.
                }
            } else if cmd.script {
                for table in tables {
                    println!("{table:-}\n");
                }
            } else {
                for table in tables {
                    info!(
                        "Table for {TERTIARY_STYLE}{}{TERTIARY_STYLE:#} runs",
                        table.body.len()
                    );
                    info!("{table}");
                }
                info!(
                    "Run with {CMD_STYLE} --output=\"path/to/location.csv\" {CMD_STYLE:#} \
                    to save to a csv file"
                );
                if csv.format.is_none() {
                    info!(
                        "Hint: use the {CMD_DOC_STYLE} --format {CMD_DOC_STYLE:#} \
                        flag to customise the table columns"
                    );
                    info!(
                        "Example: {CMD_STYLE} \
                        --format=\"program,group,wall-time,n-iv-csw\" {CMD_STYLE:#}"
                    );
                }
            }
        }

        GourdCommand::Cancel(CancelStruct {
            experiment_id,
            run_ids,
            all,
        }) => {
            let s: SlurmHandler<SlurmCli> = SlurmHandler::default();
            let experiment = read_experiment(experiment_id, cmd, &file_system)?;

            let id_list = if *all {
                s.internal.scheduled_jobs()?
            } else if let Some(ids) = run_ids {
                // verify that every id has a slurm id in the experiment
                ids.iter()
                    .map(|id| {
                        experiment
                            .runs
                            .get(*id)
                            .ok_or(anyhow!(
                                "Could not find a run with id {id} in experiment {}",
                                experiment.seq
                            ))
                            .with_context(ctx!(
                                "",;
                                "Experiment {} has runs with ids 0-{}",
                                experiment.seq, experiment.runs.len() - 1
                            ))
                            .and_then(|x| {
                                x.slurm_id
                                    .clone()
                                    .ok_or(anyhow!("Could not find run {id} on Slurm"))
                                    .with_context(ctx!(
                                        "You can only cancel runs that have been scheduled on Slurm.", ;
                                        "Run {CMD_STYLE}gourd status {}{CMD_STYLE:#} \
                                        to check which runs have been scheduled.", experiment.seq
                                    ))
                            })
                    })
                    .collect::<Result<Vec<String>>>()?
            } else {
                // get all slurm ids from the experiment
                experiment
                    .runs
                    .iter()
                    .filter_map(|run| run.slurm_id.clone())
                    .collect::<Vec<String>>()
            };

            if id_list.is_empty() {
                bailc!(
                    "No runs to cancel", ;
                    "You can only cancel runs that have been scheduled on Slurm.", ;
                     "Run {CMD_STYLE}gourd status {}{CMD_STYLE:#} to check \
                     which runs have been scheduled.", experiment.seq
                );
            }

            if cmd.dry {
                info!(
                    "Would have cancelled {TERTIARY_STYLE}[{}]{TERTIARY_STYLE:#}",
                    id_list.join(", ")
                );
            } else {
                s.internal.cancel_jobs(id_list)?;
            }
        }

        GourdCommand::Version => print_version(cmd.script),

        GourdCommand::Continue(ContinueStruct { experiment_id }) => {
            let mut experiment = read_experiment(experiment_id, cmd, &file_system)?;

            let statuses = experiment.status(&file_system)?;

            if experiment.unscheduled(&statuses).is_empty() {
                info!("Nothing more to continue :D");
                return Ok(());
            }

            // Continuing the experiment
            let exp_path = experiment.save(&file_system)?;

            if experiment.env == Environment::Local {
                if cmd.dry {
                    info!("Would have continued the experiment (dry)");
                } else {
                    let to_complete = run_local(&mut experiment, &file_system, true, false).await?;

                    info!("Experiment started");

                    // Run will never unshorten status, hence the false.
                    blocking_status(&progress, &experiment, &mut file_system, false, to_complete)?;

                    info!("Experiment finished");
                }
            } else if experiment.env == Environment::Slurm {
                let s: SlurmHandler<SlurmCli> = SlurmHandler::default();
                s.check_version()?;
                s.check_partition(&slurm_options_from_experiment(&experiment)?.partition)?;

                if cmd.dry {
                    info!("Would have continued the experiment on slurm (dry)");
                } else {
                    let sched = s.run_experiment(&mut experiment, exp_path, &file_system)?;
                    print_scheduling(&experiment, false)?;
                    info!("Experiment continued, you just scheduled {sched} chunks");
                }
            }

            let p = experiment.save(&file_system)?;
            if cmd.script {
                println!("{}", p.display());
            }
        }

        GourdCommand::Rerun(RerunOptions {
            experiment_id,
            run_ids,
        }) => {
            let mut experiment = read_experiment(experiment_id, cmd, &file_system)?;

            let selected_runs = rerun::runs::get_runs_from_rerun_options(
                run_ids,
                &mut experiment,
                &mut file_system,
                cmd.script,
            )?;

            trace!("Selected runs: {selected_runs:?}");

            // NOTE: when rerunning we should only update the limits of the new runs,
            // and not of the whole experiment.
            query_changing_resource_limits(
                &mut experiment,
                cmd.script,
                &selected_runs,
                &mut file_system,
            )?;

            for run_id in &selected_runs {
                let new_id = experiment.runs.len();
                let old_run = &experiment.runs[*run_id];

                experiment.runs.push(generate_new_run(
                    new_id,
                    old_run.program,
                    old_run.input.clone(),
                    old_run.generated_from_input.clone(),
                    old_run.group.clone(),
                    // since we still update & save the limits for every program,
                    // new resource limits are fetched from the old run's program.
                    // in a future stateless gourd, we will only update the limits for new runs,
                    // instead of the limits of the entire program.
                    experiment.programs[experiment.runs[*run_id].program].limits,
                    old_run.parent,
                    &experiment,
                    &file_system,
                )?);

                experiment.runs[*run_id].rerun = Some(new_id);
            }

            experiment.save(&file_system)?;

            if selected_runs.is_empty() {
                info!("No new runs to schedule");
                info!("Goodbye");
            } else {
                info!("{} new runs have been created", &selected_runs.len());
                info!(
                    "Run {CMD_STYLE} gourd continue {} {CMD_STYLE:#} to schedule them",
                    experiment.seq
                );
            }
        }
    }

    Ok(())
}

/// Prepare the log levels for the application.
///
/// Sets up a Colog logger with verbosity based on the flags provided by the
/// user. Valid verbosities are 0, 1, or 2 (for example, 2 is denoted by "-vv").
fn setup_logging(cmd: &Cli) -> Result<MultiProgress> {
    let mut log_build = default_builder();
    log_build.format(formatter(LogTokens));

    let bar = MultiProgress::new();

    if cmd.verbose == 2 {
        log_build.filter(None, LevelFilter::Trace);
    } else if cmd.verbose == 1 {
        log_build.filter(None, LevelFilter::Debug);
    } else if cmd.verbose == 0 {
        log_build.filter(None, LevelFilter::Info);
    } else {
        bailc!(
            "Only two levels of verbosity supported (ie. -vv)", ;
            "", ;
            "",
        )
    }

    LogWrapper::new(bar.clone(), log_build.build())
        .try_init()
        .with_context(ctx!(
            "Failed to initialize the command line interface", ;
            "Make sure you are using a supported terminal",
        ))?;

    Ok(bar)
}
