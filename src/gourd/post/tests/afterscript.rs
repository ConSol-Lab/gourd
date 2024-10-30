use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

use gourd_lib::config::UserInput;
use gourd_lib::config::UserProgram;
use tempdir::TempDir;

use crate::post::afterscript::run_afterscripts_for_experiment;
use crate::test_utils::create_sample_experiment;
use crate::test_utils::REAL_FS;

const PRE_PROGRAMMED_SH_SCRIPT: &str = r#"#!/bin/sh
echo "🩴"
"#;

#[test]
fn test_run_afterscript_for_run_good_weather() {
    let dir = TempDir::new("after_test").unwrap();
    let script_path = dir.path().join("script");
    let script_file = fs::File::create(&script_path).unwrap();
    fs::write(&script_path, PRE_PROGRAMMED_SH_SCRIPT).unwrap();
    script_file
        .set_permissions(Permissions::from_mode(0o755))
        .unwrap();
    let (mut sample, _) = create_sample_experiment(
        [(
            "ruta".into(),
            UserProgram {
                binary: Some(script_path.clone()),
                fetch: None,
                git: None,
                arguments: vec![],
                afterscript: Some(script_path.clone()),
                resource_limits: None,
                next: vec![],
            },
        )]
        .into(),
        [(
            "inp".into(),
            UserInput {
                file: None,
                glob: None,
                fetch: None,
                group: None,
                arguments: vec!["hi".into()],
            },
        )]
        .into(),
    );

    run_afterscripts_for_experiment(&mut sample, &REAL_FS).unwrap();

    assert_eq!(sample.runs[0].afterscript_output, Some("🩴".into()));
}
