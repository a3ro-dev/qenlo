pub mod app;
pub mod theme;
pub mod ui;

use std::io::stdout;
use std::time::Duration;
use crossterm::{
    event::{Event, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use crate::state::SharedState;
use self::app::App;

pub async fn run_tui(shared_state: SharedState) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(shared_state).await;
    let mut event_stream = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(100));

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if app.should_quit {
            break;
        }

        tokio::select! {
            _ = ticker.tick() => {
                // Check if status toast expired (3 seconds)
                if let Some((_, created, _)) = app.status_message {
                    if created.elapsed() > Duration::from_secs(4) {
                        app.status_message = None;
                    }
                }
            }
            Some(Ok(event)) = event_stream.next() => {
                if let Event::Key(key) = event {
                    app.handle_key(key).await;
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
