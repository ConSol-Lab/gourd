use std::process::Command;

use gourd_lib::constants::TASK_LIMIT;

use crate::local::runner::run_locally;
use crate::test_utils::get_compiled_example;

/// Run a naive fibonacci implementation using the local runner,
/// assert that they run correctly
/// Note: while there are no assertions for it, the outputs
/// should be in increasing order of computation time!
#[tokio::test]
async fn runner_fibonacci_test() {
    // Ensure binary is built
    let (out, _tmp) = get_compiled_example(include_str!("test_resources/fibonacci.rs"), None);

    let test_cases = vec![38u128, 36u128, 34u128, 30u128, 24u128];
    let mut commands: Vec<Command> = vec![];
    for value in test_cases {
        let mut cmd = Command::new(&out);
        cmd.arg(value.to_string());
        commands.push(cmd);
    }

    let results = run_locally(commands, false, false, 0).await;

    assert!(results.is_ok(), "Executing children processes failed");
}

/// Test sleeping in the thread pool (don't drown tho)
#[tokio::test]
async fn runner_sleep_test() {
    let mut commands: Vec<Command> = vec![];
    for value in [4, 3, 2, 1, 2, 3] {
        let mut cmd = Command::new("sleep");
        cmd.arg(value.to_string());
        commands.push(cmd);
    }

    let results = run_locally(commands, false, false, 0).await;

    assert!(results.is_ok(), "Executing children processes failed");
}

/// Test hitting the task limit
#[tokio::test]
async fn test_limit() {
    let mut commands: Vec<Command> = vec![];
    for _ in 0..TASK_LIMIT + 1 {
        let cmd = Command::new("sleep");
        commands.push(cmd);
    }

    let results = run_locally(commands, false, false, 0).await;

    assert!(results.is_err(), "Executing children processes failed");
}
