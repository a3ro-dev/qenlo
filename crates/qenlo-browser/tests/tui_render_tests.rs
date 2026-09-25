use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qenlo_browser::state::BrowserSession;
use qenlo_browser::tui::{
    app::{App, Tab},
    ui,
};
use ratatui::{Terminal, backend::TestBackend};
use std::sync::Arc;
use tokio::sync::RwLock;

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| ui::render(frame, app)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .await;
}

#[tokio::test]
async fn functions_tab_is_readable_at_80x24_and_filters() {
    let mut app = App::new(Arc::new(RwLock::new(BrowserSession::new()))).await;
    app.current_tab = Tab::Help;

    let full = screen(&app);
    assert!(full.contains("(33/33)"), "{full}");
    assert!(
        full.contains("new_with_options"),
        "names must not be truncated:\n{full}"
    );
    assert!(
        full.contains("let db =") && full.contains("for durable commits"),
        "summary and example must be visible at 80x24:\n{full}"
    );

    press(&mut app, KeyCode::Char('/')).await;
    for c in "batch".chars() {
        press(&mut app, KeyCode::Char(c)).await;
    }
    press(&mut app, KeyCode::Down).await;
    let filtered = screen(&app);
    assert!(filtered.contains("(3/33)"), "{filtered}");
    assert!(filtered.contains("Collection::delete_batch"), "{filtered}");
    // `q` typed into the filter must not quit the browser.
    press(&mut app, KeyCode::Char('q')).await;
    assert!(!app.should_quit);
    assert!(screen(&app).contains("no functions match"));

    press(&mut app, KeyCode::Esc).await;
    assert!(screen(&app).contains("(33/33)"));
}
