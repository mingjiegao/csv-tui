use std::io::Write;
use csv_tui::{columns::column_names, csv_loader::load_csv_to_memory, db::{execute_readonly_query, validate_readonly_sql}};

#[test]
fn imports_csv_without_headers_as_generated_columns() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file, "one,two").unwrap();
    writeln!(file, "three,four").unwrap();
    let (conn, info) = load_csv_to_memory(file.path()).unwrap();
    assert_eq!(info.columns, vec!["a", "b"]);
    assert_eq!(info.rows, 2);
    let result = execute_readonly_query(&conn, "SELECT a,b FROM csv_data ORDER BY a", 10).unwrap();
    assert_eq!(result.rows, vec![vec!["one", "two"], vec!["three", "four"]]);
}

#[test]
fn supports_many_generated_column_names() {
    assert_eq!(column_names(28)[26], "aa");
}

#[test]
fn rejects_write_sql() {
    assert!(validate_readonly_sql("DELETE FROM csv_data").is_err());
    assert!(validate_readonly_sql("SELECT * FROM csv_data").is_ok());
}
