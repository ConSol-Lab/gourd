use gourd_lib::config::UserInput;

use crate::config;
use crate::gourd;
use crate::init;
use crate::save_gourd_toml;

#[test]
fn test_analyse_csv() {
    let env = init();

    // Create a new experiment configuration in the tempdir.
    let conf = config!(&env; "fibonacci"; (
        "input_ten".to_string(),
        UserInput {
            file: None,
            arguments: vec!["10".to_string()],
        },
    ));

    // write the configuration to the tempdir
    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    let _output = gourd!(env; "-c", conf_path.to_str().unwrap(),
        "run", "local", "-s"; "dry run local");

    let _output = gourd!(env; "-c", conf_path.to_str().unwrap(),
    "analyse", "-o", "csv"; "analyse csv");

    assert!(conf.experiments_folder.join("analysis_1.csv").exists());

    let _output = gourd!(env; "-c", conf_path.to_str().unwrap(),
    "analyse", "-o", "plot-png"; "analyse png");

    assert!(conf.experiments_folder.join("plot_1.png").exists());
}
