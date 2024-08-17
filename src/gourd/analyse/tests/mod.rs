use std::time::Duration;

use gourd_lib::measurement::RUsage;

use super::*;

pub(crate) static TEST_RUSAGE: RUsage = RUsage {
    utime: Duration::from_micros(2137),
    stime: Duration::from_micros(2137),
    maxrss: 2137,
    ixrss: 2137,
    idrss: 2137,
    isrss: 2137,
    minflt: 2137,
    majflt: 2137,
    nswap: 2137,
    inblock: 2137,
    oublock: 2137,
    msgsnd: 2137,
    msgrcv: 2137,
    nsignals: 2137,
    nvcsw: 2137,
    nivcsw: 2137,
};

#[test]
fn test_table_display() {
    let table: Table<&str, 2> = Table {
        header: Some(["hello", "world"]),
        body: vec![["a", "b b b b b"], ["hi", ":)"]],
        footer: Some(["bye", ""]),
    };
    assert_eq!(
        "\
| hello | world     |
*-------*-----------*
| a     | b b b b b |
| hi    | :)        |
*-------*-----------*
| bye   |           |
",
        table.to_string()
    )
}
