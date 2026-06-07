use std::{io, path::PathBuf, time::Duration};
use anyhow::Result;
use clap::Parser;
use crossterm::{event::{self, Event, KeyCode, KeyEventKind, KeyModifiers}, execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}};
use ratatui::{Terminal, backend::CrosstermBackend};
use csv_tui::{app::{App, FocusPane}, csv_loader::load_csv_to_memory, ui};

#[derive(Parser)]
struct Args { csv_file: PathBuf }

fn main() -> Result<()> {
    let args = Args::parse();
    let (conn, info) = load_csv_to_memory(args.csv_file)?;
    let mut app = App::new(conn, info);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let res = run(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, app))?;
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), _) => app.should_quit = true,
                    (KeyCode::Tab, _) => app.toggle_focus(),
                    (KeyCode::Enter, _) => app.execute(),
                    (KeyCode::Backspace, _) if app.focus == FocusPane::Sql => app.backspace(),
                    (KeyCode::Char('f'), KeyModifiers::CONTROL) if app.focus == FocusPane::Result => app.page_down(),
                    (KeyCode::PageDown, _) if app.focus == FocusPane::Result => app.page_down(),
                    (KeyCode::Char('b'), KeyModifiers::CONTROL) if app.focus == FocusPane::Result => app.page_up(),
                    (KeyCode::PageUp, _) if app.focus == FocusPane::Result => app.page_up(),
                    (KeyCode::Up, _) if app.focus == FocusPane::Result => app.move_up(),
                    (KeyCode::Down, _) if app.focus == FocusPane::Result => app.move_down(),
                    (KeyCode::Left, _) if app.focus == FocusPane::Result => app.move_left(),
                    (KeyCode::Right, _) if app.focus == FocusPane::Result => app.move_right(),
                    (KeyCode::Left, _) if app.focus == FocusPane::Sql => app.cursor_left(),
                    (KeyCode::Right, _) if app.focus == FocusPane::Sql => app.cursor_right(),
                    (KeyCode::Char(c), _) if app.focus == FocusPane::Sql => app.input_char(c),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
