use super::*;

#[test]
fn get_slurm_options_from_config_test() {
    let config = Config {
        slurm: Some(SlurmConfig {
            partition: "test".to_string(),
            array_count_limit: 10,
            array_size_limit: 1000,
            out: None,
            experiment_name: "test".to_string(),
            account: "test-account".to_string(),
            begin: None,
            mail_type: None,
            mail_user: None,
            additional_args: None,
        }),
        ..Default::default()
    };
    assert!(slurm_options_from_experiment(&config).is_ok());
}

#[test]
fn get_slurm_options_from_config_un_test() {
    let config = Config {
        slurm: None,
        ..Default::default()
    };
    assert!(slurm_options_from_experiment(&config).is_err());
}
