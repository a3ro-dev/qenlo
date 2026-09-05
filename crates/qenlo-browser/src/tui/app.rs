use crate::state::{
    BrowserStatus, DiagnosticsDto, RecordDto, SearchHitDto, SharedState, StorageDetailsDto,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qenlo::{Filter, TimestampRange};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    DataRows,
    VectorSearch,
    StorageWal,
    Diagnostics,
    Help,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::DataRows => Tab::VectorSearch,
            Tab::VectorSearch => Tab::StorageWal,
            Tab::StorageWal => Tab::Diagnostics,
            Tab::Diagnostics => Tab::Help,
            Tab::Help => Tab::DataRows,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::DataRows => Tab::Help,
            Tab::VectorSearch => Tab::DataRows,
            Tab::StorageWal => Tab::VectorSearch,
            Tab::Diagnostics => Tab::StorageWal,
            Tab::Help => Tab::Diagnostics,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::DataRows => "1: 📋 Rows",
            Tab::VectorSearch => "2: ⚡ Search",
            Tab::StorageWal => "3: 💾 Storage",
            Tab::Diagnostics => "4: 📊 Diagnostics",
            Tab::Help => "?: ❓ Help",
        }
    }
}

pub struct App {
    pub shared_state: SharedState,
    pub current_tab: Tab,
    pub status: BrowserStatus,
    pub records: Vec<RecordDto>,
    pub selected_idx: usize,
    pub page_offset: usize,
    pub page_limit: usize,
    pub total_records: usize,

    // Modals & Inputs
    pub command_mode: bool,
    pub command_input: String,
    pub filter_mode: bool,
    pub filter_input: String,

    pub inspect_modal: Option<RecordDto>,
    pub add_modal: bool,
    pub add_field_idx: usize, // 0: ID, 1: User ID, 2: Timestamp, 3: Vector
    pub add_id: String,
    pub add_user_id: String,
    pub add_ts: String,
    pub add_vec: String,

    // Search Studio
    pub search_vec_input: String,
    pub search_k: usize,
    pub search_user_filter: String,
    pub search_results: Vec<SearchHitDto>,
    pub search_metrics: String,
    pub search_active_field: usize, // 0: Vector input, 1: k, 2: user filter

    // Storage & Diagnostics
    pub storage_details: Option<StorageDetailsDto>,
    pub diagnostics: Option<DiagnosticsDto>,

    pub status_message: Option<(String, Instant, bool)>,
    pub should_quit: bool,
}

impl App {
    pub async fn new(shared_state: SharedState) -> Self {
        let (status, records, total) = {
            let session = shared_state.read().await;
            let status = session.get_status();
            let (records, total) = match session.scan_records(0, 30, None) {
                Ok(p) => (p.records, p.total),
                Err(_) => (Vec::new(), 0),
            };
            (status, records, total)
        };

        let dim = status.dimension;
        let mut app = Self {
            shared_state,
            current_tab: Tab::DataRows,
            status,
            records,
            selected_idx: 0,
            page_offset: 0,
            page_limit: 30,
            total_records: total,
            command_mode: false,
            command_input: String::new(),
            filter_mode: false,
            filter_input: String::new(),
            inspect_modal: None,
            add_modal: false,
            add_field_idx: 0,
            add_id: "1".to_string(),
            add_user_id: "1".to_string(),
            add_ts: "1000".to_string(),
            add_vec: String::new(),
            search_vec_input: format!("[{}]", vec!["0.1"; dim.min(4)].join(", ")),
            search_k: 10,
            search_user_filter: String::new(),
            search_results: Vec::new(),
            search_metrics: String::new(),
            search_active_field: 0,
            storage_details: None,
            diagnostics: None,
            status_message: Some((
                "Welcome to QenloDB Browser. Press ? for help, : for commands.".to_string(),
                Instant::now(),
                false,
            )),
            should_quit: false,
        };

        app.refresh_storage().await;
        app.refresh_diagnostics().await;
        app
    }

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status_message = Some((msg.into(), Instant::now(), is_error));
    }

    pub async fn refresh_data(&mut self) {
        let shared = self.shared_state.clone();
        let (status, scan_res) = {
            let session = shared.read().await;
            let status = session.get_status();

            let filter = if !self.filter_input.trim().is_empty() {
                if let Ok(uid) = self.filter_input.trim().parse::<u64>() {
                    Some(Filter::new(Some(uid), TimestampRange::ALL))
                } else {
                    None
                }
            } else {
                None
            };

            let scan_res = session.scan_records(self.page_offset, self.page_limit, filter.as_ref());
            (status, scan_res)
        };

        self.status = status;
        if let Ok(p) = scan_res {
            self.records = p.records;
            self.total_records = p.total;
            if self.selected_idx >= self.records.len() {
                self.selected_idx = self.records.len().saturating_sub(1);
            }
        }
    }

    pub async fn refresh_storage(&mut self) {
        let shared = self.shared_state.clone();
        let storage = {
            let session = shared.read().await;
            session.get_storage_details()
        };
        self.storage_details = Some(storage);
    }

    pub async fn refresh_diagnostics(&mut self) {
        let shared = self.shared_state.clone();
        let diag = {
            let session = shared.read().await;
            session.get_diagnostics()
        };
        self.diagnostics = Some(diag);
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        // Global quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // Modal: Inspect Record
        if self.inspect_modal.is_some() {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                self.inspect_modal = None;
            }
            return;
        }

        // Modal: Add Record
        if self.add_modal {
            match key.code {
                KeyCode::Esc => {
                    self.add_modal = false;
                }
                KeyCode::Tab | KeyCode::Down => {
                    self.add_field_idx = (self.add_field_idx + 1) % 4;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    self.add_field_idx = (self.add_field_idx + 3) % 4;
                }
                KeyCode::Enter => {
                    self.submit_add_record().await;
                }
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.generate_random_add_vector();
                }
                KeyCode::Backspace => {
                    let field = match self.add_field_idx {
                        0 => &mut self.add_id,
                        1 => &mut self.add_user_id,
                        2 => &mut self.add_ts,
                        _ => &mut self.add_vec,
                    };
                    field.pop();
                }
                KeyCode::Char(c) => {
                    let field = match self.add_field_idx {
                        0 => &mut self.add_id,
                        1 => &mut self.add_user_id,
                        2 => &mut self.add_ts,
                        _ => &mut self.add_vec,
                    };
                    field.push(c);
                }
                _ => {}
            }
            return;
        }

        // Command Mode (Claude Code style `:`)
        if self.command_mode {
            match key.code {
                KeyCode::Esc => {
                    self.command_mode = false;
                    self.command_input.clear();
                }
                KeyCode::Enter => {
                    let cmd = self.command_input.clone();
                    self.command_mode = false;
                    self.command_input.clear();
                    self.execute_command(&cmd).await;
                }
                KeyCode::Backspace => {
                    self.command_input.pop();
                }
                KeyCode::Char(c) => {
                    self.command_input.push(c);
                }
                _ => {}
            }
            return;
        }

        // Filter Mode (`/`)
        if self.filter_mode {
            match key.code {
                KeyCode::Esc => {
                    self.filter_mode = false;
                }
                KeyCode::Enter => {
                    self.filter_mode = false;
                    self.page_offset = 0;
                    self.refresh_data().await;
                    let filter_text = self.filter_input.clone();
                    self.set_status(format!("Filter applied: '{filter_text}'"), false);
                }
                KeyCode::Backspace => {
                    self.filter_input.pop();
                }
                KeyCode::Char(c) => {
                    self.filter_input.push(c);
                }
                _ => {}
            }
            return;
        }

        // Tab Switching
        match key.code {
            KeyCode::Tab => {
                self.current_tab = self.current_tab.next();
                return;
            }
            KeyCode::BackTab => {
                self.current_tab = self.current_tab.prev();
                return;
            }
            KeyCode::Char('1') => {
                self.current_tab = Tab::DataRows;
                return;
            }
            KeyCode::Char('2') => {
                self.current_tab = Tab::VectorSearch;
                return;
            }
            KeyCode::Char('3') => {
                self.current_tab = Tab::StorageWal;
                self.refresh_storage().await;
                return;
            }
            KeyCode::Char('4') => {
                self.current_tab = Tab::Diagnostics;
                self.refresh_diagnostics().await;
                return;
            }
            KeyCode::Char('?') => {
                self.current_tab = Tab::Help;
                return;
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                return;
            }
            KeyCode::Char('/') if self.current_tab == Tab::DataRows => {
                self.filter_mode = true;
                return;
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
                return;
            }
            _ => {}
        }

        // Tab specific interactions
        match self.current_tab {
            Tab::DataRows => self.handle_rows_key(key).await,
            Tab::VectorSearch => self.handle_search_key(key).await,
            Tab::StorageWal => self.handle_storage_key(key).await,
            _ => {}
        }
    }

    async fn handle_rows_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.records.is_empty() && self.selected_idx < self.records.len() - 1 {
                    self.selected_idx += 1;
                }
            }
            KeyCode::PageUp => {
                self.selected_idx = self.selected_idx.saturating_sub(10);
            }
            KeyCode::PageDown => {
                if !self.records.is_empty() {
                    self.selected_idx = (self.selected_idx + 10).min(self.records.len() - 1);
                }
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected_idx = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                if !self.records.is_empty() {
                    self.selected_idx = self.records.len() - 1;
                }
            }
            KeyCode::Char('n') => {
                if self.page_offset + self.page_limit < self.total_records {
                    self.page_offset += self.page_limit;
                    self.selected_idx = 0;
                    self.refresh_data().await;
                }
            }
            KeyCode::Char('p') => {
                if self.page_offset > 0 {
                    self.page_offset = self.page_offset.saturating_sub(self.page_limit);
                    self.selected_idx = 0;
                    self.refresh_data().await;
                }
            }
            KeyCode::Enter => {
                if let Some(record) = self.records.get(self.selected_idx) {
                    self.inspect_modal = Some(record.clone());
                }
            }
            KeyCode::Char('a') => {
                self.add_modal = true;
                self.add_field_idx = 0;
                let next_id = self.records.iter().map(|r| r.id).max().unwrap_or(0) + 1;
                self.add_id = next_id.to_string();
                self.generate_random_add_vector();
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(record) = self.records.get(self.selected_idx) {
                    let id = record.id;
                    let shared = self.shared_state.clone();
                    let res = {
                        let session = shared.read().await;
                        session.delete_record(id)
                    };
                    match res {
                        Ok(()) => {
                            self.set_status(format!("Record #{id} deleted durably"), false);
                            self.refresh_data().await;
                        }
                        Err(e) => self.set_status(format!("Delete error: {e}"), true),
                    }
                }
            }
            KeyCode::Char('r') => {
                self.refresh_data().await;
                self.set_status("Refreshed records table", false);
            }
            _ => {}
        }
    }

    async fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.search_active_field = (self.search_active_field + 1) % 3;
            }
            KeyCode::Char('r')
                if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.is_empty() =>
            {
                self.generate_random_search_vector();
                self.set_status("Generated random query vector", false);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if self.search_k < qenlo::MAX_K {
                    self.search_k += 1;
                }
            }
            KeyCode::Char('-') => {
                if self.search_k > 1 {
                    self.search_k -= 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('s') => {
                self.execute_search().await;
            }
            KeyCode::Backspace => {
                if self.search_active_field == 0 {
                    self.search_vec_input.pop();
                } else if self.search_active_field == 2 {
                    self.search_user_filter.pop();
                }
            }
            KeyCode::Char(c) => {
                if self.search_active_field == 0 {
                    self.search_vec_input.push(c);
                } else if self.search_active_field == 2 {
                    self.search_user_filter.push(c);
                }
            }
            _ => {}
        }
    }

    async fn handle_storage_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('f') => {
                let shared = self.shared_state.clone();
                let res = {
                    let session = shared.read().await;
                    session.flush()
                };
                match res {
                    Ok(()) => self.set_status("Collection flushed and compacted to disk", false),
                    Err(e) => self.set_status(format!("Flush error: {e}"), true),
                }
                self.refresh_storage().await;
                self.refresh_data().await;
            }
            KeyCode::Char('r') => {
                self.refresh_storage().await;
                self.set_status("Storage details updated", false);
            }
            _ => {}
        }
    }

    pub fn generate_random_add_vector(&mut self) {
        let dim = self.status.dimension;
        let mut vec = Vec::with_capacity(dim);
        for _ in 0..dim {
            vec.push(format!("{:.4}", (rand_float() * 2.0 - 1.0)));
        }
        self.add_vec = format!("[{}]", vec.join(", "));
    }

    pub fn generate_random_search_vector(&mut self) {
        let dim = self.status.dimension;
        let mut vec = Vec::with_capacity(dim);
        for _ in 0..dim {
            vec.push(format!("{:.4}", (rand_float() * 2.0 - 1.0)));
        }
        self.search_vec_input = format!("[{}]", vec.join(", "));
    }

    pub async fn execute_search(&mut self) {
        let vec_str = self.search_vec_input.trim();
        let vector: Vec<f32> = if vec_str.starts_with('[') {
            serde_json::from_str(vec_str).unwrap_or_default()
        } else {
            vec_str
                .split(&[',', ' '][..])
                .filter_map(|s| s.parse::<f32>().ok())
                .collect()
        };

        if vector.is_empty() {
            self.set_status("Query vector is empty or malformed", true);
            return;
        }

        let user_id = self.search_user_filter.trim().parse::<u64>().ok();
        let filter = Filter::new(user_id, TimestampRange::ALL);

        let shared = self.shared_state.clone();
        let search_res = {
            let session = shared.read().await;
            session.search(&vector, &filter, self.search_k).await
        };

        match search_res {
            Ok(res) => {
                self.search_results = res.results;
                self.search_metrics = format!(
                    "Hits: {} · Latency: {:.2} ms · Backend: {} ({}) {}",
                    self.search_results.len(),
                    res.total_duration_us as f64 / 1000.0,
                    res.actual_backend,
                    res.algorithm,
                    res.cpu_distance_path.as_deref().unwrap_or(""),
                );
                let duration_ms = res.total_duration_us as f64 / 1000.0;
                self.set_status(format!("Search completed in {duration_ms:.2} ms"), false);
            }
            Err(e) => {
                self.set_status(format!("Search failed: {e}"), true);
            }
        }
    }

    async fn submit_add_record(&mut self) {
        let id: u64 = match self.add_id.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.set_status("Invalid ID", true);
                return;
            }
        };
        let user_id: u64 = match self.add_user_id.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.set_status("Invalid User ID", true);
                return;
            }
        };
        let ts: i64 = match self.add_ts.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.set_status("Invalid Timestamp", true);
                return;
            }
        };

        let vec_str = self.add_vec.trim();
        let vector: Vec<f32> = if vec_str.starts_with('[') {
            serde_json::from_str(vec_str).unwrap_or_default()
        } else {
            vec_str
                .split(&[',', ' '][..])
                .filter_map(|s| s.parse::<f32>().ok())
                .collect()
        };

        if vector.is_empty() {
            self.set_status("Vector components missing", true);
            return;
        }

        let shared = self.shared_state.clone();
        let insert_res = {
            let session = shared.read().await;
            session.add_record(id, user_id, ts, &vector)
        };

        match insert_res {
            Ok(()) => {
                self.set_status(format!("Record #{id} inserted successfully"), false);
                self.add_modal = false;
                self.refresh_data().await;
            }
            Err(e) => self.set_status(format!("Insert failed: {e}"), true),
        }
    }

    async fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "q" | "quit" | "exit" => {
                self.should_quit = true;
            }
            "open" => {
                if parts.len() < 2 {
                    self.set_status("Usage: :open <path> [dim]", true);
                    return;
                }
                let path = parts[1];
                let dim = parts.get(2).and_then(|s| s.parse::<usize>().ok());
                let shared = self.shared_state.clone();
                let open_res = {
                    let mut session = shared.write().await;
                    session.open_collection(path, dim).await
                };

                match open_res {
                    Ok(_) => {
                        self.set_status(format!("Opened collection: {path}"), false);
                        self.page_offset = 0;
                        self.refresh_data().await;
                        self.refresh_storage().await;
                    }
                    Err(e) => self.set_status(format!("Open error: {e}"), true),
                }
            }
            "create" => {
                if parts.len() < 3 {
                    self.set_status("Usage: :create <path> <dim>", true);
                    return;
                }
                let path = parts[1];
                let dim: usize = match parts[2].parse() {
                    Ok(d) => d,
                    Err(_) => {
                        self.set_status("Dimension must be integer", true);
                        return;
                    }
                };
                let shared = self.shared_state.clone();
                let create_res = {
                    let mut session = shared.write().await;
                    session.create_collection(path, dim).await
                };

                match create_res {
                    Ok(_) => {
                        self.set_status(
                            format!("Created collection at {path} ({dim} dims)"),
                            false,
                        );
                        self.page_offset = 0;
                        self.refresh_data().await;
                        self.refresh_storage().await;
                    }
                    Err(e) => self.set_status(format!("Create error: {e}"), true),
                }
            }
            "flush" => {
                let shared = self.shared_state.clone();
                let flush_res = {
                    let session = shared.read().await;
                    session.flush()
                };
                match flush_res {
                    Ok(()) => self.set_status("Collection flushed and compacted", false),
                    Err(e) => self.set_status(format!("Flush error: {e}"), true),
                }
                self.refresh_storage().await;
            }
            "export" => {
                if parts.len() < 2 {
                    self.set_status("Usage: :export <path.qn>", true);
                    return;
                }
                let export_path = parts[1];
                let shared = self.shared_state.clone();
                let export_res = {
                    let session = shared.read().await;
                    session.export_qn(export_path)
                };
                match export_res {
                    Ok(()) => self.set_status(format!("Exported .qn to {export_path}"), false),
                    Err(e) => self.set_status(format!("Export error: {e}"), true),
                }
            }
            "search" => {
                self.current_tab = Tab::VectorSearch;
                self.execute_search().await;
            }
            "help" => {
                self.current_tab = Tab::Help;
            }
            other => {
                self.set_status(
                    format!("Unknown command: '{other}'. Type :help for commands"),
                    true,
                );
            }
        }
    }
}

fn rand_float() -> f32 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(123456789);
    let mut x = SEED.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    (x as f32) / (u64::MAX as f32)
}
