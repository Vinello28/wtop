use crate::collectors::process::ProcessCollector;
use crate::config::{AppConfig, ProcessSortBy, ThemeMode};
use crate::model::SystemSnapshot;
use crate::updater::UpdateStatus;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Cpu,
    Memory,
    Disk,
    Gpu,
    Network,
    Processes,
}

pub struct AppState {
    pub config: AppConfig,
    pub snapshot: SystemSnapshot,
    pub active_panel: ActivePanel,
    pub proc_selected_idx: usize,
    pub proc_scroll_offset: usize,
    pub proc_filter: String,
    pub is_filtering: bool,
    pub pending_kill: Option<u32>,
    pub show_help: bool,
    pub should_quit: bool,
    pub update_status: UpdateStatus,
    pub pending_update_confirm: bool,
    pub update_confirmed: bool,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            snapshot: SystemSnapshot::default(),
            active_panel: ActivePanel::Processes,
            proc_selected_idx: 0,
            proc_scroll_offset: 0,
            proc_filter: String::new(),
            is_filtering: false,
            pending_kill: None,
            show_help: false,
            should_quit: false,
            update_status: UpdateStatus::default(),
            pending_update_confirm: false,
            update_confirmed: false,
        }
    }

    pub fn update_snapshot(&mut self, snapshot: SystemSnapshot) {
        let count = snapshot.processes.len();
        self.snapshot = snapshot;
        if count > 0 && self.proc_selected_idx >= count {
            self.proc_selected_idx = count.saturating_sub(1);
        }
        self.adjust_proc_scroll(20);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // 1. Text Filter Input Mode
        if self.is_filtering {
            match key.code {
                KeyCode::Esc => {
                    self.is_filtering = false;
                }
                KeyCode::Enter => {
                    self.is_filtering = false;
                }
                KeyCode::Backspace => {
                    self.proc_filter.pop();
                    self.proc_selected_idx = 0;
                    self.proc_scroll_offset = 0;
                }
                KeyCode::Char(c) if !c.is_control() => {
                    self.proc_filter.push(c);
                    self.proc_selected_idx = 0;
                    self.proc_scroll_offset = 0;
                }
                _ => {}
            }
            return;
        }

        // 2. Kill Confirmation Prompt Mode
        if let Some(pid) = self.pending_kill {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let _ = ProcessCollector::kill_process(pid);
                    self.pending_kill = None;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.pending_kill = None;
                }
                _ => {}
            }
            return;
        }

        // 3. Update Confirmation Prompt Mode
        if self.pending_update_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.pending_update_confirm = false;
                    self.update_confirmed = true;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.pending_update_confirm = false;
                }
                _ => {}
            }
            return;
        }

        // 4. Help Modal Mode
        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('q') => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        // 5. Standard Navigation & Shortcuts
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.show_help = true;
            }
            KeyCode::Char('u') => {
                if matches!(self.update_status, UpdateStatus::Available(_)) {
                    self.pending_update_confirm = true;
                }
            }
            KeyCode::Char('g') => {
                crate::browser::open_url("https://github.com/Vinello28");
            }
            KeyCode::Char('t') => {
                self.config.theme = match self.config.theme {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::Dark,
                };
                self.config.save();
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.config.cycle_sampling_rate(false); // faster rate (lower ms)
                self.config.save();
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.config.cycle_sampling_rate(true); // slower rate (higher ms)
                self.config.save();
            }
            KeyCode::Tab => {
                self.cycle_panel(false);
            }
            KeyCode::BackTab => {
                self.cycle_panel(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.proc_selected_idx > 0 {
                    self.proc_selected_idx -= 1;
                    self.adjust_proc_scroll(20);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.snapshot.processes.is_empty()
                    && self.proc_selected_idx + 1 < self.snapshot.processes.len()
                {
                    self.proc_selected_idx += 1;
                    self.adjust_proc_scroll(20);
                }
            }
            KeyCode::PageUp => {
                self.proc_selected_idx = self.proc_selected_idx.saturating_sub(10);
                self.adjust_proc_scroll(20);
            }
            KeyCode::PageDown => {
                if !self.snapshot.processes.is_empty() {
                    self.proc_selected_idx =
                        (self.proc_selected_idx + 10).min(self.snapshot.processes.len() - 1);
                    self.adjust_proc_scroll(20);
                }
            }
            KeyCode::Home => {
                self.proc_selected_idx = 0;
                self.proc_scroll_offset = 0;
            }
            KeyCode::End => {
                if !self.snapshot.processes.is_empty() {
                    self.proc_selected_idx = self.snapshot.processes.len() - 1;
                    self.adjust_proc_scroll(20);
                }
            }
            KeyCode::Char('c') => {
                if self.config.proc_sort_by == ProcessSortBy::Cpu {
                    self.config.proc_sort_desc = !self.config.proc_sort_desc;
                } else {
                    self.config.proc_sort_by = ProcessSortBy::Cpu;
                    self.config.proc_sort_desc = true;
                }
                self.config.save();
            }
            KeyCode::Char('m') => {
                if self.config.proc_sort_by == ProcessSortBy::Memory {
                    self.config.proc_sort_desc = !self.config.proc_sort_desc;
                } else {
                    self.config.proc_sort_by = ProcessSortBy::Memory;
                    self.config.proc_sort_desc = true;
                }
                self.config.save();
            }
            KeyCode::Char('p') => {
                if self.config.proc_sort_by == ProcessSortBy::Pid {
                    self.config.proc_sort_desc = !self.config.proc_sort_desc;
                } else {
                    self.config.proc_sort_by = ProcessSortBy::Pid;
                    self.config.proc_sort_desc = false;
                }
                self.config.save();
            }
            KeyCode::Char('n') => {
                if self.config.proc_sort_by == ProcessSortBy::Name {
                    self.config.proc_sort_desc = !self.config.proc_sort_desc;
                } else {
                    self.config.proc_sort_by = ProcessSortBy::Name;
                    self.config.proc_sort_desc = false;
                }
                self.config.save();
            }
            KeyCode::Char('d') => {
                self.config.proc_sort_desc = !self.config.proc_sort_desc;
                self.config.save();
            }
            KeyCode::Char('/') => {
                self.is_filtering = true;
            }
            KeyCode::Esc => {
                if !self.proc_filter.is_empty() {
                    self.proc_filter.clear();
                    self.proc_selected_idx = 0;
                    self.proc_scroll_offset = 0;
                }
            }
            KeyCode::Char('x') | KeyCode::Char('K') | KeyCode::Delete => {
                if let Some(target) = self.snapshot.processes.get(self.proc_selected_idx) {
                    self.pending_kill = Some(target.pid);
                }
            }
            _ => {}
        }
    }

    fn cycle_panel(&mut self, reverse: bool) {
        self.active_panel = match (self.active_panel, reverse) {
            (ActivePanel::Cpu, false) => ActivePanel::Memory,
            (ActivePanel::Memory, false) => ActivePanel::Disk,
            (ActivePanel::Disk, false) => ActivePanel::Gpu,
            (ActivePanel::Gpu, false) => ActivePanel::Network,
            (ActivePanel::Network, false) => ActivePanel::Processes,
            (ActivePanel::Processes, false) => ActivePanel::Cpu,

            (ActivePanel::Cpu, true) => ActivePanel::Processes,
            (ActivePanel::Processes, true) => ActivePanel::Network,
            (ActivePanel::Network, true) => ActivePanel::Gpu,
            (ActivePanel::Gpu, true) => ActivePanel::Disk,
            (ActivePanel::Disk, true) => ActivePanel::Memory,
            (ActivePanel::Memory, true) => ActivePanel::Cpu,
        };
    }

    fn adjust_proc_scroll(&mut self, visible_rows: usize) {
        if self.proc_selected_idx < self.proc_scroll_offset {
            self.proc_scroll_offset = self.proc_selected_idx;
        } else if self.proc_selected_idx >= self.proc_scroll_offset + visible_rows {
            self.proc_scroll_offset = self.proc_selected_idx.saturating_sub(visible_rows - 1);
        }
    }
}
