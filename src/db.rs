use anyhow::{Result, bail};
use rusqlite::{Connection, types::ValueRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn validate_readonly_sql(sql: &str) -> Result<()> {
    let s = sql.trim();
    if s.is_empty() { bail!("SQL is empty"); }
    let lower = s.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or("");
    if !matches!(first, "select" | "with") { bail!("only SELECT/WITH readonly queries are allowed"); }
    let banned = ["insert", "update", "delete", "create", "drop", "alter", "attach", "detach", "pragma", "replace", "vacuum", "reindex"];
    for word in banned {
        if lower.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').any(|t| t == word) {
            bail!("readonly mode rejects keyword: {word}");
        }
    }
    Ok(())
}

pub fn execute_readonly_query(conn: &Connection, sql: &str, max_rows: usize) -> Result<QueryResult> {
    validate_readonly_sql(sql)?;
    if !conn.is_readonly(rusqlite::DatabaseName::Main)? { /* memory db is writable for import; SQL guard enforces readonly user queries */ }
    let mut stmt = conn.prepare(sql)?;
    if !stmt.readonly() { bail!("statement is not readonly"); }
    let columns = stmt.column_names().into_iter().map(ToOwned::to_owned).collect::<Vec<_>>();
    let col_count = stmt.column_count();
    let mut rows_out = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        if rows_out.len() >= max_rows { break; }
        let mut vals = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let text = match row.get_ref(i)? {
                ValueRef::Null => "NULL".to_string(),
                ValueRef::Integer(v) => v.to_string(),
                ValueRef::Real(v) => v.to_string(),
                ValueRef::Text(v) => String::from_utf8_lossy(v).into_owned(),
                ValueRef::Blob(v) => format!("<{} bytes>", v.len()),
            };
            vals.push(text);
        }
        rows_out.push(vals);
    }
    Ok(QueryResult { columns, rows: rows_out })
}
