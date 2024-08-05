use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use gourd_lib::config::Config;
use gourd_lib::config::FetchedPath;
use gourd_lib::config::UserProgramMap;

use super::*;

#[test]
fn test_get_program_from_name_simple() {
    let mut config = Config::default();
    let mut program = UserProgram {
        binary: FetchedPath(PathBuf::from("fake_path")),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
        resource_limits: None,
    };
    config.programs.insert("name".to_string(), program.clone());

    let mut exp = Experiment {
        runs: vec![],
        chunks: vec![],
        resource_limits: None,
        creation_time: chrono::offset::Local::now(),
        config,
        seq: 0,
        env: gourd_lib::experiment::Environment::Local,
        postprocess_inputs: BTreeMap::new(),
    };

    let prog = get_program_from_name(&mut exp, &"name".to_string());
    assert!(prog.is_ok());
    assert_eq!(prog.unwrap(), &mut program);
}

#[test]
fn test_get_program_from_name_post() {
    let mut config = Config::default();
    let mut program = UserProgram {
        binary: FetchedPath(PathBuf::from("fake_path")),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
        resource_limits: None,
    };

    let mut post_progs = UserProgramMap::default();
    post_progs.insert("name".to_string(), program.clone());
    config.postprocess_programs = Some(post_progs);

    let mut exp = Experiment {
        runs: vec![],
        chunks: vec![],
        resource_limits: None,
        creation_time: chrono::offset::Local::now(),
        config,
        seq: 0,
        env: gourd_lib::experiment::Environment::Local,
        postprocess_inputs: BTreeMap::new(),
    };

    let prog = get_program_from_name(&mut exp, &"name".to_string());
    assert!(prog.is_ok());
    assert_eq!(prog.unwrap(), &mut program);
}

#[test]
fn test_get_program_from_name_error() {
    let mut exp = Experiment {
        runs: vec![],
        chunks: vec![],
        resource_limits: None,
        creation_time: chrono::offset::Local::now(),
        config: Config::default(),
        seq: 0,
        env: gourd_lib::experiment::Environment::Local,
        postprocess_inputs: BTreeMap::new(),
    };

    assert!(get_program_from_name(&mut exp, &"fake_name".to_string()).is_err());
}

#[test]
fn test_query_changing_limits_for_program() {
    let mut config = Config::default();
    let program = UserProgram {
        binary: FetchedPath(PathBuf::from("fake_path")),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
        resource_limits: None,
    };

    config.programs.insert("name1".to_string(), program.clone());

    let mut exp = Experiment {
        runs: vec![],
        chunks: vec![],
        resource_limits: None,
        creation_time: chrono::offset::Local::now(),
        config,
        seq: 0,
        env: gourd_lib::experiment::Environment::Local,
        postprocess_inputs: BTreeMap::new(),
    };

    assert!(query_changing_limits_for_program(
        &"name1".to_string(),
        false,
        &mut exp,
        Some(20),
        Some(2),
        Some(Duration::from_secs(45))
    )
    .is_ok());

    let prog = get_program_from_name(&mut exp, &"name1".to_string());
    let lims = prog.unwrap().resource_limits.unwrap();

    assert_eq!(lims.mem_per_cpu, 20);
    assert_eq!(lims.cpus, 2);
    assert_eq!(lims.time_limit, Duration::from_secs(45));
}

#[test]
fn test_query_changing_limits_for_all_programs() {
    let programs = vec!["name1".to_string(), "name2".to_string()];

    let mut config = Config::default();
    let program = UserProgram {
        binary: FetchedPath(PathBuf::from("fake_path")),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
        resource_limits: None,
    };

    config.programs.insert("name1".to_string(), program.clone());

    let mut post_progs = UserProgramMap::default();
    post_progs.insert("name2".to_string(), program.clone());
    config.postprocess_programs = Some(post_progs);

    let mut exp = Experiment {
        runs: vec![],
        chunks: vec![],
        resource_limits: None,
        creation_time: chrono::offset::Local::now(),
        config,
        seq: 0,
        env: gourd_lib::experiment::Environment::Local,
        postprocess_inputs: BTreeMap::new(),
    };

    let old_rss = ResourceLimits::default();
    let new_rss = ResourceLimits {
        mem_per_cpu: 20,
        cpus: 2,
        time_limit: Duration::from_secs(45),
    };

    assert!(query_changing_limits_for_all_programs(&mut exp, new_rss, &old_rss).is_ok());

    for program in programs {
        let prog = get_program_from_name(&mut exp, &program);
        let lims = prog.unwrap().resource_limits.unwrap();
        assert_eq!(lims.mem_per_cpu, 20);
        assert_eq!(lims.cpus, 2);
        assert_eq!(lims.time_limit, Duration::from_secs(45));
    }
}

#[test]
fn test_get_setlim_programs() {
    let mut config = Config::default();
    let program = UserProgram {
        binary: FetchedPath(PathBuf::from("fake_path")),
        arguments: vec![],
        afterscript: None,
        postprocess_job: None,
        resource_limits: None,
    };

    config.programs.insert("name1".to_string(), program.clone());
    config.programs.insert("name2".to_string(), program.clone());

    let mut post_progs = UserProgramMap::default();
    post_progs.insert("name_post".to_string(), program.clone());
    config.postprocess_programs = Some(post_progs);

    let exp = Experiment {
        runs: vec![],
        chunks: vec![],
        resource_limits: None,
        creation_time: chrono::offset::Local::now(),
        config,
        seq: 0,
        env: gourd_lib::experiment::Environment::Local,
        postprocess_inputs: BTreeMap::new(),
    };

    let prog = get_setlim_programs(&exp).unwrap();
    assert_eq!(prog.len(), 3);
}
