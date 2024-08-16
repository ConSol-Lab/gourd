use super::*;
use crate::config::SubParameter;

#[test]
fn test_expand_parameters_ok_no_expandable() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "first".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec!["nice".to_string()],
        },
    );

    let parameters = BTreeMap::new();
    let expanded = expand_parameters(inputs.clone(), &parameters).unwrap();
    assert_eq!(expanded, inputs);
}

#[test]
fn test_expand_parameters_ok_parameter() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "first".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec!["-e".to_string(), "param|x".to_string()],
        },
    );
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "x".to_string(),
        Parameter {
            sub: None,
            values: Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
        },
    );
    let expanded = expand_parameters(inputs.clone(), &parameters).unwrap();
    let mut expected = BTreeMap::new();
    expected.insert(
        format!("first_x_0{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec!["-e".to_string(), "a".to_string()],
        },
    );
    expected.insert(
        format!("first_x_1{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec!["-e".to_string(), "b".to_string()],
        },
    );
    expected.insert(
        format!("first_x_2{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec!["-e".to_string(), "c".to_string()],
        },
    );

    assert_eq!(expanded, expected);
}

#[test]
fn test_expand_parameters_ok_parameter_doubled() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "first".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "param|x".to_string(),
                "-f".to_string(),
                "param|x".to_string(),
            ],
        },
    );
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "x".to_string(),
        Parameter {
            sub: None,
            values: Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
        },
    );
    let expanded = expand_parameters(inputs.clone(), &parameters).unwrap();
    let mut expected = BTreeMap::new();
    expected.insert(
        format!("first_x_0{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "a".to_string(),
                "-f".to_string(),
                "a".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_1{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "b".to_string(),
                "-f".to_string(),
                "b".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_2{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "c".to_string(),
                "-f".to_string(),
                "c".to_string(),
            ],
        },
    );
    assert_eq!(expanded, expected);
}

#[test]
fn test_expand_parameters_ok_subparameter() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "first".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "subparam|x.1".to_string(),
                "-x".to_string(),
                "subparam|x.2".to_string(),
            ],
        },
    );
    let mut sub_parameters = BTreeMap::new();
    sub_parameters.insert(
        "1".to_string(),
        SubParameter {
            values: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        },
    );
    sub_parameters.insert(
        "2".to_string(),
        SubParameter {
            values: vec!["10".to_string(), "20".to_string(), "30".to_string()],
        },
    );
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "x".to_string(),
        Parameter {
            sub: Some(sub_parameters),
            values: None,
        },
    );
    let expanded = expand_parameters(inputs, &parameters).unwrap();
    let mut expected = BTreeMap::new();
    expected.insert(
        format!("first_x_0{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "a".to_string(),
                "-x".to_string(),
                "10".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_1{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "b".to_string(),
                "-x".to_string(),
                "20".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_2{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "c".to_string(),
                "-x".to_string(),
                "30".to_string(),
            ],
        },
    );
    assert_eq!(expanded, expected);
}

#[test]
fn test_expand_parameters_ok_both() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "first".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "subparam|x.1".to_string(),
                "-f".to_string(),
                "param|y".to_string(),
                "-x".to_string(),
                "subparam|x.2".to_string(),
            ],
        },
    );
    let mut sub_parameters = BTreeMap::new();
    sub_parameters.insert(
        "1".to_string(),
        SubParameter {
            values: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        },
    );
    sub_parameters.insert(
        "2".to_string(),
        SubParameter {
            values: vec!["10".to_string(), "20".to_string(), "30".to_string()],
        },
    );

    let mut parameters = BTreeMap::new();
    parameters.insert(
        "x".to_string(),
        Parameter {
            sub: Some(sub_parameters),
            values: None,
        },
    );
    parameters.insert(
        "y".to_string(),
        Parameter {
            sub: None,
            values: Some(vec![
                "xxx".to_string(),
                "yyy".to_string(),
                "zzz".to_string(),
            ]),
        },
    );

    let expanded = expand_parameters(inputs, &parameters).unwrap();
    let mut expected = BTreeMap::new();
    expected.insert(
        format!("first_x_0_y_0{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "a".to_string(),
                "-f".to_string(),
                "xxx".to_string(),
                "-x".to_string(),
                "10".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_1_y_0{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "b".to_string(),
                "-f".to_string(),
                "xxx".to_string(),
                "-x".to_string(),
                "20".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_2_y_0{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "c".to_string(),
                "-f".to_string(),
                "xxx".to_string(),
                "-x".to_string(),
                "30".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_0_y_1{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "a".to_string(),
                "-f".to_string(),
                "yyy".to_string(),
                "-x".to_string(),
                "10".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_1_y_1{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "b".to_string(),
                "-f".to_string(),
                "yyy".to_string(),
                "-x".to_string(),
                "20".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_2_y_1{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "c".to_string(),
                "-f".to_string(),
                "yyy".to_string(),
                "-x".to_string(),
                "30".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_0_y_2{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "a".to_string(),
                "-f".to_string(),
                "zzz".to_string(),
                "-x".to_string(),
                "10".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_1_y_2{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "b".to_string(),
                "-f".to_string(),
                "zzz".to_string(),
                "-x".to_string(),
                "20".to_string(),
            ],
        },
    );
    expected.insert(
        format!("first_x_2_y_2{INTERNAL_PREFIX}{INTERNAL_PARAMETER}"),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "c".to_string(),
                "-f".to_string(),
                "zzz".to_string(),
                "-x".to_string(),
                "30".to_string(),
            ],
        },
    );
    assert_eq!(expanded, expected);
}

#[test]
fn test_expand_parameters_fail_subparameter_not_declared() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "first".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec!["-e".to_string(), "param|x".to_string()],
        },
    );
    let parameters = BTreeMap::new();
    assert!(expand_parameters(inputs, &parameters).is_err());
}

#[test]
fn test_expand_parameters_fail_subparameter_size_not_match() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "first".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![
                "-e".to_string(),
                "param|x_1".to_string(),
                "-x".to_string(),
                "param|x_2".to_string(),
            ],
        },
    );
    let mut sub_parameters = BTreeMap::new();
    sub_parameters.insert(
        "1".to_string(),
        SubParameter {
            values: vec!["a".to_string(), "b".to_string()],
        },
    );
    sub_parameters.insert(
        "2".to_string(),
        SubParameter {
            values: vec!["10".to_string(), "20".to_string(), "30".to_string()],
        },
    );
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "x".to_string(),
        Parameter {
            sub: Some(sub_parameters),
            values: None,
        },
    );
    assert!(expand_parameters(inputs, &parameters).is_err());
}
