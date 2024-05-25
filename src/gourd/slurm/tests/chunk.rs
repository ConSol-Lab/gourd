use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use gourd_lib::config::Input;
use gourd_lib::config::Program;

use super::*;
use crate::test_utils::create_sample_experiment;

#[test]
fn get_unscheduled_runs_test() {
    let prog = Program {
        binary: PathBuf::new(),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
    };

    let input = Input {
        input: None,
        arguments: vec![],
    };

    let (mut experiment, _conf) = create_sample_experiment(
        BTreeMap::from([
            (String::from("Prog1"), prog.clone()),
            (String::from("Prog2"), prog.clone()),
            (String::from("Prog3"), prog),
        ]),
        BTreeMap::from([
            (String::from("Inp1"), input.clone()),
            (String::from("Inp2"), input.clone()),
            (String::from("Inp3"), input),
        ]),
    );

    let resource_limits = ResourceLimits {
        time_limit: Duration::new(600, 0),
        cpus: 0,
        mem_per_cpu: 0,
    };

    experiment.slurm = Some(SlurmExperiment {
        chunks: vec![Chunk {
            runs: vec![0, 1],
            resource_limits: resource_limits.clone(),
        }],
        resource_limits,
    });

    let runs = experiment.get_unscheduled_runs().unwrap();

    assert_eq!(runs, vec!(2, 3, 4, 5, 6, 7, 8))
}

#[test]
fn create_chunks_basic_test() {
    let prog = Program {
        binary: PathBuf::new(),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
    };

    let input = Input {
        input: None,
        arguments: vec![],
    };

    let (mut experiment, _conf) = create_sample_experiment(
        BTreeMap::from([
            (String::from("Prog1"), prog.clone()),
            (String::from("Prog2"), prog.clone()),
            (String::from("Prog3"), prog),
        ]),
        BTreeMap::from([
            (String::from("Inp1"), input.clone()),
            (String::from("Inp2"), input.clone()),
            (String::from("Inp3"), input),
        ]),
    );

    let resource_limits = ResourceLimits {
        time_limit: Duration::new(600, 0),
        cpus: 0,
        mem_per_cpu: 0,
    };

    experiment.slurm = Some(SlurmExperiment {
        chunks: vec![Chunk {
            runs: vec![0, 1],
            resource_limits: resource_limits.clone(),
        }],
        resource_limits: resource_limits.clone(),
    });
    let chunks = experiment
        .create_chunks(3, 2, experiment.get_unscheduled_runs().unwrap().into_iter())
        .unwrap();

    assert_eq!(
        chunks,
        vec!(
            Chunk {
                runs: vec![2, 3, 4],
                resource_limits: resource_limits.clone(),
            },
            Chunk {
                runs: vec![5, 6, 7],
                resource_limits: resource_limits.clone(),
            }
        )
    )
}

#[test]
fn create_chunks_greedy_test() {
    let prog_a = Program {
        binary: PathBuf::new().join("a"),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
    };

    let prog_b = Program {
        binary: PathBuf::new().join("b"),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
    };

    let input_a = Input {
        input: Some(PathBuf::new().join("a")),
        arguments: vec![],
    };

    let input_b = Input {
        input: Some(PathBuf::new().join("b")),
        arguments: vec![],
    };

    let (mut experiment, _conf) = create_sample_experiment(
        BTreeMap::from([
            (String::from("Prog_A"), prog_a.clone()),
            (String::from("Prog_B"), prog_b.clone()),
        ]),
        {
            let mut inputs = BTreeMap::new();
            for i in 0..5 {
                _ = inputs.insert(format!("Input_A_{}", i), input_a.clone())
            }
            for i in 0..5 {
                _ = inputs.insert(format!("Input_B_{}", i), input_b.clone())
            }
            inputs
        },
    );

    let resource_limits_a = ResourceLimits {
        time_limit: Duration::new(600, 0),
        cpus: 0,
        mem_per_cpu: 0,
    };

    let resource_limits_b = ResourceLimits {
        time_limit: Duration::new(1200, 0),
        cpus: 0,
        mem_per_cpu: 0,
    };

    experiment.slurm = Some(SlurmExperiment {
        chunks: vec![Chunk {
            runs: vec![],
            resource_limits: resource_limits_b.clone(),
        }],
        resource_limits: resource_limits_b.clone(),
    });

    // Gets all 20 runs
    let mut runs = experiment.get_unscheduled_runs().unwrap().into_iter();

    // Mapping function:
    // - use limits_A for combination of input_A and program_A
    // - use limits_B for everything else
    let f = |r: &Run| {
        if r.input.starts_with("Input_A") && r.program == "Prog_A" {
            return resource_limits_a.clone();
        }
        resource_limits_b.clone()
    };

    // Test greedy algorithm
    let mut chunks_greedy = experiment
        .create_chunks_with_resource_limits(6, 2, f, runs.clone())
        .unwrap();
    assert_eq!(
        vec!(
            // Postpones filling Runs 0-4 as they have
            // different limits, and the array would not be full
            Chunk {
                // Runs 5-9: prog_A, input_B
                // Run 10: prog_B, input_A (same limits)
                runs: vec![5, 6, 7, 8, 9, 10],
                resource_limits: resource_limits_b.clone(),
            },
            Chunk {
                // Run 11-14: prog_B, input_A
                // Run 15: prog_B, input_B (same limits)
                runs: vec![11, 12, 13, 14, 15, 16],
                resource_limits: resource_limits_b.clone(),
            }
        ),
        chunks_greedy
    );

    // Test basic algorithm
    let chunks_basic = experiment.create_chunks(6, 2, runs.clone()).unwrap();
    assert_eq!(
        vec!(
            // Does not use mapping function, so just takes the first runs
            Chunk {
                runs: vec![0, 1, 2, 3, 4, 5],
                resource_limits: resource_limits_b.clone(),
            },
            Chunk {
                runs: vec![6, 7, 8, 9, 10, 11],
                resource_limits: resource_limits_b.clone(),
            }
        ),
        chunks_basic,
    );

    // Test the rest of the unscheduled runs!
    experiment
        .slurm
        .as_mut()
        .unwrap()
        .chunks
        .append(&mut chunks_greedy);
    runs = experiment.get_unscheduled_runs().unwrap().into_iter();
    assert_eq!(
        runs.clone().collect::<Vec<usize>>(),
        vec!(0, 1, 2, 3, 4, 17, 18, 19)
    );

    chunks_greedy = experiment
        .create_chunks_with_resource_limits(6, 2, f, runs.clone())
        .unwrap();
    assert_eq!(
        vec!(
            // Finishes scheduling runs 0-4 (len = 5), then 16-19 (len = 4)
            Chunk {
                // Runs 0-4: prog_A, input_A (special limits!)
                // The chunk has 5/6 runs (not full)
                runs: vec![0, 1, 2, 3, 4],
                resource_limits: resource_limits_a.clone(),
            },
            Chunk {
                // Run 11-14: prog_B, input_A
                // Run 15: prog_B, input_B (same limits)
                runs: vec![17, 18, 19],
                resource_limits: resource_limits_b.clone(),
            }
        ),
        chunks_greedy,
    );
}