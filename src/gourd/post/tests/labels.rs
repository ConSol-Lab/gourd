// TODO: fix

// #[test]
// fn test_add_label_to_run() {
//     let fs = FileSystemInteractor { dry_run: true };
//     let dir = TempDir::new("config_folder").expect("A temp folder could not
// be created.");     let file_pb = dir.path().join("file.toml");
//     let config_contents = r#"
//              output_path = "./goose"
//              metrics_path = "./🪿/"
//              experiments_folder = "/tmp/gourd/experiments/"
//              [program.a]
//              binary = "/bin/sleep"
//              arguments = []
//              afterscript = "/bin/echo"
//              [input.b]
//              arguments = ["1"]
//              [input.c]
//              arguments = ["2"]
//              [label.found_hello]
//              priority = 0
//              regex = "hello"
//              [label.found_world]
//              priority = 1
//              regex = "world"
//          "#;
//     let mut file = File::create(file_pb.as_path()).expect("A file could not
// be created.");     file.write_all(config_contents.as_bytes())
//         .expect("The test file could not be written.");
//     let mut after_file =
//         File::create(dir.path().join("after.txt")).expect("A file could not
// be created.");     after_file
//         .write_all("hello".as_bytes())
//         .expect("The test file could not be written.");
//
//     let conf = Config::from_file(file_pb.as_path(), &fs).unwrap();
//     let exp =
//         Experiment::from_config(&conf, chrono::Local::now(),
// Environment::Local, &fs).unwrap();     assert!(conf.labels.is_some());
//     assert_eq!(
//         assign_label(&exp, &dir.path().join("after.txt"), &fs).expect("tested
// fn failed"),         Some("found_hello".to_string())
//     );
//
//     after_file
//         .write_all("hello world".as_bytes())
//         .expect("The test file could not be written.");
//
//     assert_eq!(
//         assign_label(&exp, &dir.path().join("after.txt"), &fs).expect("tested
// fn failed"),         Some("found_world".to_string())
//     );
// }
