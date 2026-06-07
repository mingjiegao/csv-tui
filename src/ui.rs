use ratatui::{Frame, layout::{Constraint, Direction, Layout, Position}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, BorderType, Borders, Paragraph, Wrap}};
use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let chunks = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(4), Constraint::Length(1)])
        .split(frame.area());
    app.set_page_size(chunks[1].height.saturating_sub(3) as usize);
    let header = format!("csv-tui | table: {} | imported rows: {} | columns: {} | q/Ctrl-C quit | Enter run | arrows scroll", app.info.table_name, app.info.rows, app.info.columns.join(","));
    frame.render_widget(Paragraph::new(header).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title("CSV")), chunks[0]);

    let inner_width = chunks[1].width.saturating_sub(2) as usize;
    let mut all_widths = app.result.columns.iter().skip(app.col_offset).enumerate().map(|(idx, name)| {
        let max_cell = app.result.rows.iter().skip(app.row_offset).take(50)
            .filter_map(|row| row.get(app.col_offset + idx))
            .map(|v| v.chars().count())
            .max()
            .unwrap_or(0);
        name.chars().count().max(max_cell).min(40).max(1)
    }).collect::<Vec<_>>();
    let visible_count = visible_column_count(&all_widths, inner_width);
    all_widths.truncate(visible_count);
    expand_last_column(&mut all_widths, inner_width);
    let cols = app.result.columns.iter().skip(app.col_offset).take(visible_count).cloned().collect::<Vec<_>>();
    let result_border = if app.focus == FocusPane::Result { Color::Cyan } else { Color::DarkGray };
    let result_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(result_border)).title("Result [Tab focus]");
    let mut lines = Vec::new();
    lines.push(compact_line(&cols, &all_widths, inner_width, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    for (visible_idx, row) in app.result.rows.iter().skip(app.row_offset).enumerate() {
        let absolute_idx = app.row_offset + visible_idx;
        let values = row.iter().skip(app.col_offset).take(visible_count).cloned().collect::<Vec<_>>();
        let selected = app.selected_row == Some(absolute_idx);
        let style = if selected {
            Style::default().bg(if app.focus == FocusPane::Result { Color::Blue } else { Color::DarkGray }).fg(Color::White).add_modifier(Modifier::BOLD)
        } else { Style::default() };
        lines.push(compact_line(&values, &all_widths, inner_width, style));
    }
    frame.render_widget(Paragraph::new(lines).block(result_block), chunks[1]);

    let sql_border = if app.focus == FocusPane::Sql { Color::Cyan } else { Color::DarkGray };
    let sql_line = sql_input_line(app);
    frame.render_widget(Paragraph::new(sql_line).wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(sql_border)).title("SQL [Enter run]")), chunks[2]);
    draw_sql_signature(frame, chunks[2]);
    if app.focus == FocusPane::Sql {
        let cursor_x = chunks[2].x + 1 + app.sql[..app.cursor].chars().count() as u16;
        let max_x = chunks[2].x + chunks[2].width.saturating_sub(2);
        frame.set_cursor_position(Position { x: cursor_x.min(max_x), y: chunks[2].y + 1 });
    }
    let default_status = "Tab: switch panes | SQL: autocomplete table/columns, separators accept unique match, Enter runs | Result: arrows scroll/select, Ctrl-F/PageDown, Ctrl-B/PageUp | q quits";
    let status = app.error.as_deref().or(app.status.as_deref()).unwrap_or(default_status);
    let color = if app.error.is_some() { Color::Red } else { Color::Green };
    frame.render_widget(Paragraph::new(Line::from(status)).style(Style::default().fg(color)), chunks[3]);
}

fn compact_line(values: &[String], widths: &[usize], max_width: usize, style: Style) -> Line<'static> {
    let mut out = String::new();
    for (i, value) in values.iter().enumerate() {
        if i > 0 { out.push(' '); }
        let width = widths.get(i).copied().unwrap_or(1);
        let text = fit_cell(value, width);
        out.push_str(&text);
        let pad = width.saturating_sub(text.chars().count());
        out.push_str(&" ".repeat(pad));
        if out.chars().count() >= max_width { break; }
    }
    if out.chars().count() > max_width {
        out = out.chars().take(max_width).collect();
    }
    Line::from(Span::styled(out, style))
}

fn fit_cell(value: &str, width: usize) -> String {
    let count = value.chars().count();
    if count <= width { return value.to_string(); }
    if width <= 1 { return "…".to_string(); }
    value.chars().take(width - 1).collect::<String>() + "…"
}

fn visible_column_count(widths: &[usize], max_width: usize) -> usize {
    let mut used = 0usize;
    let mut count = 0usize;
    for width in widths {
        let need = *width + usize::from(count > 0);
        if used + need > max_width {
            break;
        }
        used += need;
        count += 1;
    }
    count.max(1).min(widths.len())
}

fn expand_last_column(widths: &mut [usize], max_width: usize) {
    if widths.is_empty() { return; }
    let used = widths.iter().sum::<usize>() + widths.len().saturating_sub(1);
    if used < max_width {
        if let Some(last) = widths.last_mut() {
            *last += max_width - used;
        }
    }
}

fn sql_input_line(app: &App) -> Line<'static> {
    let mut spans = Vec::new();
    let cursor = app.cursor.min(app.sql.len());
    let before = app.sql[..cursor].to_string();
    let after = app.sql[cursor..].to_string();
    spans.push(Span::raw(before));
    if let Some(c) = &app.completion {
        if c.token_end == cursor && !c.suffix.is_empty() {
            spans.push(Span::styled(c.suffix.clone(), Style::default().fg(Color::DarkGray)));
        }
    }
    spans.push(Span::raw(after));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_query_columns_use_full_width_by_expanding_last_column() {
        let mut widths = vec![27, 7, 40];
        let visible = visible_column_count(&widths, 158);
        widths.truncate(visible);
        expand_last_column(&mut widths, 158);
        assert_eq!(visible, 3);
        assert_eq!(widths, vec![27, 7, 122]);
    }
}

fn draw_sql_signature(frame: &mut Frame<'_>, rect: ratatui::layout::Rect) {
    let signature = "author: mingjiegao";
    let width = signature.chars().count() as u16;
    if rect.width <= width + 4 { return; }
    let x = rect.x + rect.width - width - 2;
    let area = ratatui::layout::Rect { x, y: rect.y, width, height: 1 };
    frame.render_widget(Paragraph::new(signature).style(Style::default().fg(Color::DarkGray)), area);
}
