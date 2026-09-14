mod app;
mod browser;
mod collectors;
mod config;
mod model;
mod theme;
mod ui;
mod updater;

use std::io::{Result, stdout};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::AppState;
use crate::collectors::SystemCollector;
use crate::config::AppConfig;
use crate::theme::Theme;
use crate::updater::UpdateStatus;

fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
        original_hook(panic_info);
    }));
}

fn main() -> Result<()> {
    setup_panic_hook();

    // Enable raw mode and alternate screen
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let config = AppConfig::load();
    let sampling_rate = Arc::new(AtomicU64::new(config.sampling_rate_ms));
    let is_running = Arc::new(AtomicBool::new(true));
    let filter_query = Arc::new(Mutex::new(String::new()));

    // Self-update: a genuine one-shot check, never part of the recurring
    // sampling loop below.
    let update_status = Arc::new(Mutex::new(UpdateStatus::default()));
    updater::cleanup_previous_update();
    updater::spawn_check(Arc::clone(&update_status));

    // Metrics channel: background worker -> UI thread
    let (tx_snapshot, rx_snapshot) = mpsc::sync_channel(2);

    // Spawn low-overhead background collector thread
    let bg_running = Arc::clone(&is_running);
    let bg_rate = Arc::clone(&sampling_rate);
    let bg_filter = Arc::clone(&filter_query);

    let _collector_handle = thread::Builder::new()
        .name("wtop-collector".to_string())
        .spawn(move || {
            let mut collector = SystemCollector::new();
            let mut cfg = AppConfig::default();

            while bg_running.load(Ordering::Relaxed) {
                let interval_ms = bg_rate.load(Ordering::Relaxed);
                cfg.sampling_rate_ms = interval_ms;

                let filter_str = {
                    let guard = bg_filter.lock().unwrap();
                    guard.clone()
                };

                let start = Instant::now();
                let snapshot = collector.collect(&cfg, &filter_str);

                // Send latest snapshot (drop previous if receiver is busy)
                let _ = tx_snapshot.try_send(snapshot);

                let elapsed = start.elapsed();
                let target = Duration::from_millis(interval_ms);
                if target > elapsed {
                    let sleep_time = target - elapsed;
                    // Sleep in small increments to be responsive to quit or rate changes
                    let mut remaining = sleep_time;
                    while remaining > Duration::from_millis(50)
                        && bg_running.load(Ordering::Relaxed)
                    {
                        thread::sleep(Duration::from_millis(50));
                        remaining = remaining.saturating_sub(Duration::from_millis(50));
                    }
                    if remaining > Duration::ZERO && bg_running.load(Ordering::Relaxed) {
                        thread::sleep(remaining);
                    }
                }
            }
        })
        .expect("Failed to spawn collector thread");

    let mut state = AppState::new(config);

    // Initial render
    let theme = Theme::from_mode(state.config.theme);
    terminal.draw(|f| {
        let area = f.area();
        ui::layout::render_ui(f.buffer_mut(), area, &state, &theme);
    })?;

    // Main UI Event loop
    while !state.should_quit {
        while let Ok(snap) = rx_snapshot.try_recv() {
            state.update_snapshot(snap);
        }

        // Keep sampling rate synchronized with background thread
        if state.config.sampling_rate_ms != sampling_rate.load(Ordering::Relaxed) {
            sampling_rate.store(state.config.sampling_rate_ms, Ordering::Relaxed);
        }

        // Keep process filter string synchronized with background collector
        {
            let mut guard = filter_query.lock().unwrap();
            if *guard != state.proc_filter {
                *guard = state.proc_filter.clone();
            }
        }

        // Mirror the update status down from the background checker/applier,
        // and act on a confirmed apply request.
        if let Ok(guard) = update_status.lock() {
            state.update_status = guard.clone();
        }
        if state.update_confirmed {
            state.update_confirmed = false;
            if let UpdateStatus::Available(info) = state.update_status.clone() {
                updater::spawn_apply_update(info, Arc::clone(&update_status));
            }
        }
        if matches!(state.update_status, UpdateStatus::Ready) {
            // The new version has been swapped in and relaunched; let the
            // existing shutdown path run so the terminal is left sane
            // before this process exits.
            state.should_quit = true;
        }

        // Render frame
        let current_theme = Theme::from_mode(state.config.theme);
        terminal.draw(|f| {
            let area = f.area();
            ui::layout::render_ui(f.buffer_mut(), area, &state, &current_theme);
        })?;

        // Handle terminal inputs with a short timeout for buttery smoothness
        if event::poll(Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        state.handle_key(key);
                    }
                }
                Event::Resize(_, _) => {
                    terminal.autoresize()?;
                }
                _ => {}
            }
        }
    }

    // Clean shutdown
    is_running.store(false, Ordering::Relaxed);
    state.config.save();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    Ok(())
}
