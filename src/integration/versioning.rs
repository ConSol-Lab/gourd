// use flate2::bufread::GzDecoder;
// use tar::Archive;
//
// use crate::config;
// use crate::gourd;
// use crate::init;
//
// #[test]
// fn test_repo_commit() {
//     let env = init();
//
//     let gz =
// GzDecoder::new(&include_bytes!("../resources/test_repo.tar.gz")[..]);     let
// mut archive = Archive::new(gz);     archive.unpack(&env.temp_dir).unwrap();
//
//     let _ = config(&env, "./src/integration/configurations/git.toml");
//
//     gourd!(env; "run", "local"; "failed to use repo versioning");
// }
