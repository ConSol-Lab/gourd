use std::path::PathBuf;

use flate2::bufread::GzDecoder;
use gourd_lib::config::GitProgram;
use gourd_lib::config::UserProgram;
use tar::Archive;

use crate::config;
use crate::gourd;
use crate::init;
use crate::save_gourd_toml;

#[test]
fn test_repo_commit() {
    let env = init();

    let gz = GzDecoder::new(&include_bytes!("../resources/test_repo.tar.gz")[..]);
    let mut archive = Archive::new(gz);
    archive.unpack(&env.temp_dir).unwrap();

    let mut conf = config!(&env; ; );
    conf.programs.insert(
        "test".to_string(),
        UserProgram {
            binary: None,
            git: Some(GitProgram {
                commit_id: "07566620bd74d3f57dd9d0ef5a9cc8681b210659".to_string(),
                build_command: "cp test.sh run.sh".to_string(),
                path: PathBuf::from("run.sh"),
                git_uri: "./repo/".to_string(),
            }),
            fetch: None,
            arguments: vec![],
            afterscript: None,
            resource_limits: None,
            next: vec![],
        },
    );

    save_gourd_toml(&conf, &env.temp_dir);
    gourd!(env; "run", "local"; "failed to use repo versioning");
}
