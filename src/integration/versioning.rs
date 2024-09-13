use std::path::PathBuf;

use flate2::bufread::GzDecoder;
use gourd_lib::config::GitResource;
use gourd_lib::config::UserProgram;
use tar::Archive;
use tempdir::TempDir;

use crate::config;
use crate::gourd;
use crate::init;
use crate::save_gourd_toml;

#[test]
fn test_repo_commit() {
    let mut env = init();

    // Unpacks submodule_test.tar.gz
    let gz = GzDecoder::new(&include_bytes!("../resources/submodule_test.tar.gz")[..]);
    let mut archive = Archive::new(gz);
    archive.unpack(&env.temp_dir).unwrap();

    // The archive contents:
    // submodule_test
    // ├── experiment_repo              // the 'experiment repository' (workdir)
    // │   ├── first_file
    // │   └── referenced_repo          // 'remote' as a cloned submodule
    // │      └── test.sh               // note no compile script (it is in a
    // different commit) |
    // |
    // └── referenced_repo              // the 'remote' repository
    //    ├── test.sh
    //    └── compile.sh

    // a hack to effectively 'cd' into 'experiment_repo' within the integration test
    // environment
    let old_tempdir = env.temp_dir;
    env.temp_dir = TempDir::new_in(&old_tempdir, "temp_workdir").unwrap();
    let new_dir_path = env.temp_dir.path();
    std::fs::remove_dir(new_dir_path).unwrap();
    std::fs::rename(old_tempdir.path().join("experiment_repo"), new_dir_path).unwrap();

    // This configuration should get 'test.sh' from the submodule
    // by referencing the commit "56dfc35 - New commit"
    let mut conf = config!(&env; ; );
    conf.programs.insert(
        "test".to_string(),
        UserProgram {
            binary: None,
            git: Some(GitResource {
                submodule_name: "referenced_repo".to_string(),
                rev: Some("56dfc35".to_string()),
                compile_script: Some("compile.sh".to_string()),
                file: PathBuf::from("compiled_program.sh"),
                store: PathBuf::from("fetched/program.sh"),
                enable_fetch: false,
            }),
            fetch: None,
            env: None,
            arguments: vec![],
            afterscript: None,
            resource_limits: None,
            next: vec![],
        },
    );

    //     It would be nice to test *fetching* the newest version of the submodule,
    //     but libgit2 does not allow fetching from a remote that isn't actually
    //     HTTP / SSH. We don't want real remote dependencies for tests, so testing
    //     submodule fetching is left as an exercise for the reader.

    save_gourd_toml(&conf, &env.temp_dir);
    gourd!(env; "run", "local"; "failed to use repo versioning");
}
