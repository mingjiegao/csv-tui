use rusqlite::Connection;
use crate::csv_loader::CsvImportInfo;
use crate::db::{QueryResult, execute_readonly_query};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Result,
    Sql,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub token_start: usize,
    pub token_end: usize,
    pub candidate: String,
    pub suffix: String,
}

pub struct App {
    pub conn: Connection,
    pub info: CsvImportInfo,
    pub sql: String,
    pub result: QueryResult,
    pub error: Option<String>,
    pub row_offset: usize,
    pub col_offset: usize,
    pub page_size: usize,
    pub should_quit: bool,
    pub focus: FocusPane,
    pub selected_row: Option<usize>,
    pub cursor: usize,
    pub completion: Option<Completion>,
    pub status: Option<String>,
}

impl App {
    pub fn new(conn: Connection, info: CsvImportInfo) -> Self {
        let sql = "SELECT * FROM csv_data LIMIT 100".to_string();
        let result = execute_readonly_query(&conn, &sql, 1000).unwrap_or(QueryResult { columns: vec![], rows: vec![] });
        let cursor = sql.len();
        let selected_row = if result.rows.is_empty() { None } else { Some(0) };
        let mut app = Self { conn, info, sql, result, error: None, row_offset: 0, col_offset: 0, page_size: 10, should_quit: false, focus: FocusPane::Sql, selected_row, cursor, completion: None, status: None };
        app.refresh_completion();
        app
    }
    pub fn execute(&mut self) {
        match execute_readonly_query(&self.conn, &self.sql, 1000) {
            Ok(r) => { self.result = r; self.error = None; self.status = Some(format!("{} row(s)", self.result.rows.len())); self.row_offset = 0; self.col_offset = 0; self.selected_row = if self.result.rows.is_empty() { None } else { Some(0) }; }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus { FocusPane::Result => FocusPane::Sql, FocusPane::Sql => FocusPane::Result };
    }
    pub fn input_char(&mut self, c: char) {
        if is_completion_separator(c) { self.accept_completion(); }
        self.sql.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.refresh_completion();
    }
    pub fn backspace(&mut self) {
        if self.cursor == 0 { return; }
        if let Some((idx, _)) = self.sql[..self.cursor].char_indices().last() {
            self.sql.drain(idx..self.cursor);
            self.cursor = idx;
        }
        self.refresh_completion();
    }
    pub fn cursor_left(&mut self) {
        if self.cursor == 0 { return; }
        if let Some((idx, _)) = self.sql[..self.cursor].char_indices().last() { self.cursor = idx; }
        self.refresh_completion();
    }
    pub fn cursor_right(&mut self) {
        if self.cursor >= self.sql.len() { return; }
        let next = self.sql[self.cursor..].chars().next().map_or(0, char::len_utf8);
        self.cursor = (self.cursor + next).min(self.sql.len());
        self.refresh_completion();
    }
    pub fn accept_completion(&mut self) {
        let Some(c) = self.completion.clone() else { return; };
        if c.suffix.is_empty() { return; }
        self.sql.replace_range(c.token_start..c.token_end, &c.candidate);
        self.cursor = c.token_start + c.candidate.len();
        self.status = Some(format!("completed: {}", c.candidate));
        self.completion = None;
    }
    pub fn refresh_completion(&mut self) {
        let (start, token) = current_token(&self.sql, self.cursor);
        if token.is_empty() { self.completion = None; return; }
        let mut candidates = Vec::with_capacity(self.info.columns.len() + 1);
        candidates.push(self.info.table_name.clone());
        candidates.extend(self.info.columns.iter().cloned());
        let matches = candidates.into_iter().filter(|c| c.starts_with(&token)).collect::<Vec<_>>();
        match matches.as_slice() {
            [candidate] if candidate != &token => {
                self.completion = Some(Completion { token_start: start, token_end: self.cursor, suffix: candidate[token.len()..].to_string(), candidate: candidate.clone() });
                self.status = None;
            }
            [] => { self.completion = None; }
            _ => {
                self.completion = None;
                self.status = Some(format!("completions: {}", matches.iter().take(10).cloned().collect::<Vec<_>>().join(" ")));
            }
        }
    }
    pub fn move_down(&mut self) {
        let len = self.result.rows.len();
        if len == 0 { self.selected_row = None; return; }
        let next = self.selected_row.map_or(0, |i| (i + 1).min(len - 1));
        self.selected_row = Some(next);
        if next >= self.row_offset + 1 { self.row_offset = next; }
    }
    pub fn move_up(&mut self) {
        let Some(i) = self.selected_row else { return; };
        let next = i.saturating_sub(1);
        self.selected_row = Some(next);
        self.row_offset = self.row_offset.min(next);
    }
    pub fn page_down(&mut self) {
        let len = self.result.rows.len();
        if len == 0 { self.selected_row = None; return; }
        let current = self.selected_row.unwrap_or(0);
        let next = current.saturating_add(self.page_size).min(len - 1);
        self.selected_row = Some(next);
        self.row_offset = next;
    }
    pub fn page_up(&mut self) {
        let Some(current) = self.selected_row else { return; };
        let next = current.saturating_sub(self.page_size);
        self.selected_row = Some(next);
        self.row_offset = next;
    }
    pub fn set_page_size(&mut self, page_size: usize) { self.page_size = page_size.max(1); }
    pub fn move_right(&mut self) { self.col_offset = self.col_offset.saturating_add(1); }
    pub fn move_left(&mut self) { self.col_offset = self.col_offset.saturating_sub(1); }
}

fn is_completion_separator(c: char) -> bool { matches!(c, ' ' | ',' | ')' | ';' | '\n') }

fn current_token(sql: &str, cursor: usize) -> (usize, String) {
    let cursor = cursor.min(sql.len());
    let start = sql[..cursor].char_indices().rev()
        .find(|(_, c)| !is_token_char(*c))
        .map_or(0, |(idx, c)| idx + c.len_utf8());
    (start, sql[start..cursor].to_string())
}

fn is_token_char(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' }
