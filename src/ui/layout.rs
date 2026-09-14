use crate::app::{ActivePanel, AppState};
use crate::theme::Theme;
use crate::ui::cpu_panel::render_cpu_panel;
use crate::ui::disk_panel::render_disk_panel;
use crate::ui::gpu_panel::render_gpu_panel;
use crate::ui::header::render_header;
use crate::ui::help_modal::render_help_modal;
use crate::ui::mem_panel::render_mem_panel;
use crate::ui::net_panel::render_net_panel;
use crate::ui::proc_panel::{ProcPanelView, render_proc_panel};
use crate::ui::update_modal::render_update_confirm;
use crate::updater::UpdateStatus;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn render_ui(buf: &mut Buffer, area: Rect, state: &AppState, theme: &Theme) {
    if area.width < 30 || area.height < 10 {
        return;
    }

    // Header (1 line) + Main workspace
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    render_header(
        buf,
        main_chunks[0],
        &state.config,
        theme,
        state.snapshot.uptime_secs,
        &state.update_status,
    );

    let workspace = main_chunks[1];

    if workspace.width >= 100 {
        render_wide_layout(buf, workspace, state, theme);
    } else if workspace.width >= 64 {
        render_medium_layout(buf, workspace, state, theme);
    } else {
        render_narrow_layout(buf, workspace, state, theme);
    }

    // Modal popup if help requested
    if state.show_help {
        render_help_modal(buf, area, theme);
    }

    // Update confirm prompt is app-wide (triggerable from any focused
    // panel), so it's a small centered modal rather than an inline overlay.
    if state.pending_update_confirm
        && let UpdateStatus::Available(info) = &state.update_status
    {
        render_update_confirm(buf, area, theme, info);
    }
}

/// Desktop / widescreen: a 3-column x 2-row grid (CPU / MEMORY / DISK on
/// top, GPU / NETWORK / PROCESSES below) so the process list -- previously
/// a single 60%-wide, 55%-tall quadrant -- now gets one third of the width
/// and 45% of the height, like every other panel.
fn render_wide_layout(buf: &mut Buffer, workspace: Rect, state: &AppState, theme: &Theme) {
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(workspace);

    let col_constraints = [
        Constraint::Percentage(34),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ];

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_constraints)
        .split(v_chunks[0]);

    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_constraints)
        .split(v_chunks[1]);

    render_cpu_panel(
        buf,
        top[0],
        &state.snapshot.cpu,
        theme,
        state.active_panel == ActivePanel::Cpu,
    );
    render_mem_panel(
        buf,
        top[1],
        &state.snapshot.memory,
        theme,
        state.active_panel == ActivePanel::Memory,
    );
    render_disk_panel(
        buf,
        top[2],
        &state.snapshot.io,
        theme,
        state.active_panel == ActivePanel::Disk,
    );

    render_gpu_panel(
        buf,
        bot[0],
        &state.snapshot.gpu,
        theme,
        state.active_panel == ActivePanel::Gpu,
    );
    render_net_panel(
        buf,
        bot[1],
        &state.snapshot.net,
        theme,
        state.active_panel == ActivePanel::Network,
    );
    render_proc_panel(
        buf,
        bot[2],
        &proc_panel_view(state),
        theme,
        state.active_panel == ActivePanel::Processes,
    );
}

/// Medium width: CPU spans the top (it needs room for per-core bars), then
/// two 2-up rows (MEMORY/DISK, GPU/NETWORK), then PROCESSES full-width but
/// height-limited at the bottom.
fn render_medium_layout(buf: &mut Buffer, workspace: Rect, state: &AppState, theme: &Theme) {
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ])
        .split(workspace);

    render_cpu_panel(
        buf,
        v_chunks[0],
        &state.snapshot.cpu,
        theme,
        state.active_panel == ActivePanel::Cpu,
    );

    let mem_disk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(v_chunks[1]);
    render_mem_panel(
        buf,
        mem_disk[0],
        &state.snapshot.memory,
        theme,
        state.active_panel == ActivePanel::Memory,
    );
    render_disk_panel(
        buf,
        mem_disk[1],
        &state.snapshot.io,
        theme,
        state.active_panel == ActivePanel::Disk,
    );

    let gpu_net = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(v_chunks[2]);
    render_gpu_panel(
        buf,
        gpu_net[0],
        &state.snapshot.gpu,
        theme,
        state.active_panel == ActivePanel::Gpu,
    );
    render_net_panel(
        buf,
        gpu_net[1],
        &state.snapshot.net,
        theme,
        state.active_panel == ActivePanel::Network,
    );

    render_proc_panel(
        buf,
        v_chunks[3],
        &proc_panel_view(state),
        theme,
        state.active_panel == ActivePanel::Processes,
    );
}

/// Narrow terminals: a single vertical stack of just the panels that can
/// show something useful at this width (CPU, MEMORY, DISK, PROCESSES) --
/// GPU/NETWORK are dropped, same trade-off the old compact layout already
/// made below its (wider) threshold.
fn render_narrow_layout(buf: &mut Buffer, workspace: Rect, state: &AppState, theme: &Theme) {
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(24),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(40),
        ])
        .split(workspace);

    render_cpu_panel(
        buf,
        v_chunks[0],
        &state.snapshot.cpu,
        theme,
        state.active_panel == ActivePanel::Cpu,
    );
    render_mem_panel(
        buf,
        v_chunks[1],
        &state.snapshot.memory,
        theme,
        state.active_panel == ActivePanel::Memory,
    );
    render_disk_panel(
        buf,
        v_chunks[2],
        &state.snapshot.io,
        theme,
        state.active_panel == ActivePanel::Disk,
    );
    render_proc_panel(
        buf,
        v_chunks[3],
        &proc_panel_view(state),
        theme,
        state.active_panel == ActivePanel::Processes,
    );
}

fn proc_panel_view(state: &AppState) -> ProcPanelView<'_> {
    ProcPanelView {
        processes: &state.snapshot.processes,
        selected_idx: state.proc_selected_idx,
        scroll_offset: state.proc_scroll_offset,
        sort_by: state.config.proc_sort_by,
        desc: state.config.proc_sort_desc,
        filter: &state.proc_filter,
        is_filtering: state.is_filtering,
        pending_kill: state.pending_kill,
    }
}
