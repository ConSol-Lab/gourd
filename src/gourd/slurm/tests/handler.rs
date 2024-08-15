use std::collections::BTreeMap;

use gourd_lib::config::SBatchArg;
use gourd_lib::config::SlurmConfig;

use super::*;

#[test]
fn parse_optional_args_test_all() {
    let config = SlurmConfig {
        experiment_name: "test experiment".to_string(),
        output_folder: Default::default(),
        partition: "memory".to_string(),
        array_size_limit: None,
        array_count_limit: 100,
        account: "test-account".to_string(),
        begin: Some("01:10:00".to_string()),
        mail_type: Some("ALL".to_string()),
        mail_user: Some("testUSER".to_string()),
        additional_args: None,
    };
    let output = parse_optional_args(&config);
    let desired_output = "#SBATCH --begin=01:10:00
#SBATCH --mail-type=ALL
#SBATCH --mail-user=testUSER
";

    assert_eq!(output, desired_output)
}

#[test]
fn parse_optional_args_test_only_begin() {
    let config = SlurmConfig {
        experiment_name: "test experiment".to_string(),
        output_folder: Default::default(),
        partition: "memory".to_string(),
        array_size_limit: None,
        array_count_limit: 100,
        account: "test-account".to_string(),
        begin: Some("15:40:15".to_string()),
        mail_type: None,
        mail_user: None,
        additional_args: None,
    };
    let output = parse_optional_args(&config);
    let desired_output = "#SBATCH --begin=15:40:15\n";

    assert_eq!(output, desired_output)
}

#[test]
fn parse_optional_args_test_custom_args() {
    let mut custom_args_map: BTreeMap<String, SBatchArg> = BTreeMap::new();
    custom_args_map.insert(
        "a".to_string(),
        SBatchArg {
            name: "custom-arg".to_string(),
            value: "value".to_string(),
        },
    );
    custom_args_map.insert(
        "b".to_string(),
        SBatchArg {
            name: "second-custom-arg".to_string(),
            value: "second-value".to_string(),
        },
    );
    let config = SlurmConfig {
        experiment_name: "test experiment".to_string(),
        output_folder: Default::default(),
        partition: "memory".to_string(),
        array_size_limit: None,
        array_count_limit: 100,
        account: "test-account".to_string(),
        begin: None,
        mail_type: Some("ALL".to_string()),
        mail_user: Some("testUSER".to_string()),
        additional_args: Some(custom_args_map),
    };
    let output = parse_optional_args(&config);
    let desired_output = "#SBATCH --mail-type=ALL
#SBATCH --mail-user=testUSER
#SBATCH --custom-arg=value
#SBATCH --second-custom-arg=second-value
";

    assert_eq!(output, desired_output)
}
