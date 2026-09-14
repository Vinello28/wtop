use crate::app::{ActivePanel, AppState};
use crate::theme::Theme;
use crate::ui::cpu_panel::render_cpu_panel;
use crate::ui::header::render_header;
use crate::ui::help_modal::render_help_modal;
use crate::ui::mem_disk_panel::render_mem_disk_panel;
use crate::ui::net_gpu_panel::render_net_gpu_panel;
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
        // Desktop / Widescreen: 2x2 grid
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(workspace);

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(v_chunks[0]);

        let bot_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(v_chunks[1]);

        render_cpu_panel(
            buf,
            top_chunks[0],
            &state.snapshot.cpu,
            theme,
            state.active_panel == ActivePanel::Cpu,
        );
        render_mem_disk_panel(
            buf,
            top_chunks[1],
            &state.snapshot.memory,
            &state.snapshot.io,
            theme,
            state.active_panel == ActivePanel::MemDisk,
        );
        render_net_gpu_panel(
            buf,
            bot_chunks[0],
            &state.snapshot.gpu,
            &state.snapshot.net,
            theme,
            state.active_panel == ActivePanel::NetGpu,
        );
        render_proc_panel(
            buf,
            bot_chunks[1],
            &proc_panel_view(state),
            theme,
            state.active_panel == ActivePanel::Processes,
        );
    } else {
        // Compact layout: 3 vertical sections
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(30),
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
        render_mem_disk_panel(
            buf,
            v_chunks[1],
            &state.snapshot.memory,
            &state.snapshot.io,
            theme,
            state.active_panel == ActivePanel::MemDisk,
        );
        render_proc_panel(
            buf,
            v_chunks[2],
            &proc_panel_view(state),
            theme,
            state.active_panel == ActivePanel::Processes,
        );
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
