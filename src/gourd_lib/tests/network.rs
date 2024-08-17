use std::fs;

// use std::io::Read;
// use std::path::PathBuf;
use tempdir::TempDir;

use super::*;
// use crate::test_utils::REAL_FS;

pub const PREPROGRAMMED_SH_SCRIPT: &str = r#"
#!/bin/bash
cat <<EOF >filename
first line
second line
third line
EOF
"#;

#[test]
fn test_get_resources() {
    let tmp_dir = TempDir::new("testing").unwrap();
    let file_path = tmp_dir.path().join("test.sh");

    let tmp_file = File::create(&file_path).unwrap();
    fs::write(&file_path, PREPROGRAMMED_SH_SCRIPT).unwrap();

    let res = get_resources(vec![&file_path]);
    assert!(res.is_ok());
    assert_eq!(res.unwrap().len(), 1);

    drop(tmp_file);
    assert!(tmp_dir.close().is_ok());
}

// #[test] // TODO: uncomment when i have wifi again
// fn test_downloading_from_url() {
//     let output_name = "rustup-init.sh";
//     let tmp_dir = TempDir::new("testing").unwrap();
//     let file_path = tmp_dir.path().join(output_name);
//
//     let tmp_dir_path = PathBuf::from(tmp_dir.path());
//     println!("{:?}", tmp_dir_path);
//
//     download_file(
//         "https://sh.rustup.rs",
//         &tmp_dir_path.join(output_name),
//         &REAL_FS,
//     )
//     .unwrap();
//
//     let mut file = File::open(file_path).expect("could not open the file");
//     let mut contents = String::new();
//     file.read_to_string(&mut contents)
//         .expect("can't read file contents");
//
//     let text_start: String = contents.chars().take(8).collect();
//     assert_eq!("#!/bin/s", text_start);
//
//     assert!(tmp_dir.close().is_ok());
// }
