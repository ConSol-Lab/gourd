// use anyhow::anyhow;
// use anyhow::Context;
// use anyhow::Result;
// use gourd_lib::config::FetchedPath;
// use gourd_lib::config::UserInput;
// use gourd_lib::ctx;
// use gourd_lib::error::Ctx;
// use gourd_lib::experiment::Experiment;
// use gourd_lib::experiment::FieldRef;
// use gourd_lib::file_system::FileOperations;
// use log::debug;
//
// use crate::experiments::generate_new_run;
// use crate::status::ExperimentStatus;
//
// /// Schedules the postprocessing job for jobs that are completed and do not
// yet /// have a postprocess job output.
// pub fn schedule_post_jobs(
//     experiment: &mut Experiment,
//     statuses: &mut ExperimentStatus,
//     fs: &impl FileOperations,
// ) -> Result<()> {
//     let runs = filter_runs_for_post_job(statuses, experiment)?;
//
//     for run_id in &runs {
//         let run = &experiment.runs[*run_id];
//         let res_path = run.output_path.clone();
//
//         debug!("Adding postprocessing run for job {run_id}");
//
//         let program = &experiment.get_program(run)?;
//
//         let postprocess = program
//             .postprocess_job
//             .clone()
//             .ok_or(anyhow!("Could not get the postprocessing information"))
//             .with_context(ctx!(
//                 "Could not get the postprocessing information", ;
//                 "",
//             ))?;
//
//         let prog_name = run.program.clone();
//
//         let new_input_name = format!("{}_{}", prog_name, run.input);
//
//         experiment.postprocess_inputs.insert(
//             new_input_name.clone(),
//             UserInput {
//                 input: Some(FetchedPath(res_path.to_path_buf())),
//                 arguments: vec![],
//             },
//         );
//
//         let new_index = runs.len();
//         experiment.runs.push(generate_new_run(
//             new_index,
//             FieldRef::Postprocess(postprocess),
//             FieldRef::Postprocess(new_input_name),
//             experiment.seq,
//             &experiment.config,
//             fs,
//         )?);
//
//         experiment.runs[*run_id].postprocessor = Some(new_index);
//     }
//
//     Ok(())
// }
//
// // /// Finds the completed jobs where posprocess job did not run yet.
// // pub fn filter_runs_for_post_job(
// //     runs: &mut ExperimentStatus,
// //     exp: &Experiment,
// // ) -> Result<Vec<usize>> {
// //     let mut filtered = vec![];
// //
// //     for (run_id, status) in runs {
// //         if status.fs_status.completion.has_succeeded()
// //             && status.fs_status.completion.is_completed()
// //             && exp.runs[*run_id].postprocessor.is_none()
// //             && exp
// //                 .get_program(&exp.runs[*run_id])?
// //                 .postprocess_job
// //                 .is_some()
// //         {
// //             filtered.push(*run_id);
// //         }
// //     }
// //
// //     Ok(filtered)
// // }
