use std::fs;
use std::fs::remove_file;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use tempdir::TempDir;

use crate::resources::run_script;

pub const PREPROGRAMMED_SH_SCRIPT: &str = r#"
#!/bin/bash
cat <<EOF >filename
first line
second line
third line
EOF
"#;

#[test]
#[cfg(unix)]
fn test_sh_script() {
    let tmp_dir = TempDir::new("testing").unwrap();
    let file_path = tmp_dir.path().join("test.sh");

    let tmp_file = File::create(&file_path).unwrap();
    fs::write(&file_path, PREPROGRAMMED_SH_SCRIPT).unwrap();

    let res = run_script(
        "sh",
        vec!["-C", &(file_path.into_os_string().to_str().unwrap())],
        tmp_dir.path(),
    );
    assert!(res.is_ok());

    let full_path = tmp_dir.path().join("./filename");
    assert!(Path::exists(&full_path));

    let mut file = File::open(&full_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let text_start: String = contents.chars().take(10).collect();
    assert_eq!("first line", text_start);

    remove_file(&full_path).unwrap();

    drop(tmp_file);
    assert!(tmp_dir.close().is_ok());
}
