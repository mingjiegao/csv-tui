use std::path::Path;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params_from_iter};
use crate::columns::column_names;

#[derive(Debug, Clone)]
pub struct CsvImportInfo {
    pub table_name: String,
    pub columns: Vec<String>,
    pub rows: usize,
}

pub fn load_csv_to_memory(path: impl AsRef<Path>) -> Result<(Connection, CsvImportInfo)> {
    let path = path.as_ref();
    let mut reader = csv::ReaderBuilder::new().has_headers(false).flexible(true).from_path(path)
        .with_context(|| format!("failed to open CSV {}", path.display()))?;
    let mut records = Vec::new();
    let mut width = 0usize;
    for rec in reader.records() {
        let rec = rec.with_context(|| format!("failed to read CSV {}", path.display()))?;
        width = width.max(rec.len());
        records.push(rec.iter().map(ToOwned::to_owned).collect::<Vec<_>>());
    }
    if width == 0 { bail!("CSV has no columns: {}", path.display()); }
    let columns = column_names(width);
    let conn = Connection::open_in_memory().context("failed to open in-memory sqlite")?;
    let defs = columns.iter().map(|c| format!("\"{c}\" TEXT")).collect::<Vec<_>>().join(", ");
    conn.execute(&format!("CREATE TABLE csv_data ({defs})"), [])?;
    let marks = (0..width).map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!("INSERT INTO csv_data VALUES ({marks})");
    {
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(&sql)?;
            for mut row in records {
                row.resize(width, String::new());
                stmt.execute(params_from_iter(row.iter()))?;
            }
        }
        tx.commit()?;
    }
    let rows = conn.query_row("SELECT COUNT(*) FROM csv_data", [], |r| r.get::<_, i64>(0))? as usize;
    Ok((conn, CsvImportInfo { table_name: "csv_data".into(), columns, rows }))
}
