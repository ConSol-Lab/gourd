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
    let table: Table = Table {
        columns: 2,
        header: Some(vec!["hello".into(), "world".into()]),
        body: vec![
            vec!["a".into(), "b b b b b".into()],
            vec!["hi".into(), ":)".into()],
        ],
        footer: Some(vec!["bye".into(), "".into()]),
    };
    assert_eq!(
        "
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

#[test]
fn test_table_column_widths() {
    let table: Table = Table {
        columns: 2,
        header: Some(vec!["hallo".into(), "world".into()]),
        body: vec![
            vec!["a".into(), "b b b b b".into()],
            vec!["hi".into(), ":)".into()],
        ],
        footer: Some(vec!["bye".into(), "".into()]),
    };
    assert_eq!(vec![5, 9], table.column_widths())
}

#[test]
fn test_appending_columns() {
    let column: Column = Column {
        header: Some("hello".into()),
        body: vec!["a".into(), "b b b b b".into()],
        footer: Some("bye".into()),
    };
    let mut table: Table = Table {
        columns: 1,
        header: Some(vec!["hello".into()]),
        body: vec![vec!["a".into()], vec!["hi".into()]],
        footer: Some(vec!["bye".into()]),
    };
    table.append_column(column);

    assert_eq!(
        "
| hello | hello     |
*-------*-----------*
| a     | a         |
| hi    | b b b b b |
*-------*-----------*
| bye   | bye       |
",
        table.to_string()
    )
}
