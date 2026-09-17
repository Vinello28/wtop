use crate::app::{ActivePanel, AppState};
use crate::model::{BatteryData, DiskIoData};
use crate::theme::Theme;
use crate::ui::battery_panel::render_battery_panel;
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

/// Fixed height (borders included) given to the battery panel when it's
/// carved out of the bottom of the disk panel's cell.
const BATTERY_PANEL_HEIGHT: u16 = 7;
/// Disk needs at least this much room left over to still look reasonable
/// (I/O line + a couple of chart rows + at least one partition); below
/// `DISK_MIN_HEIGHT + BATTERY_PANEL_HEIGHT` the battery panel is skipped
/// entirely and disk keeps the whole cell, same trade-off already made for
/// GPU/NETWORK in the narrow layout below.
const DISK_MIN_HEIGHT: u16 = 8;

/// Splits a disk panel's cell into a (possibly smaller) disk area and an
/// optional battery area stacked directly below it -- reclaiming space the
/// disk panel usually leaves blank, since its content rarely fills a whole
/// grid cell. Returns `(disk_area, None)` unchanged when there's no battery
/// to show or not enough spare height to show it decently.
fn split_disk_and_battery(area: Rect, battery_present: bool) -> (Rect, Option<Rect>) {
    if !battery_present || area.height < DISK_MIN_HEIGHT + BATTERY_PANEL_HEIGHT {
        return (area, None);
    }
    let disk_h = area.height - BATTERY_PANEL_HEIGHT;
    let disk_area = Rect {
        height: disk_h,
        ..area
    };
    let battery_area = Rect {
        y: area.y + disk_h,
        height: BATTERY_PANEL_HEIGHT,
        ..area
    };
    (disk_area, Some(battery_area))
}

/// Renders the disk panel into `area`, carving off a battery panel below it
/// when there's room and a battery is present. Shared by all three layout
/// tiers so the space-reclaiming logic lives in one place.
fn render_disk_and_battery(
    buf: &mut Buffer,
    area: Rect,
    io: &DiskIoData,
    battery: &BatteryData,
    theme: &Theme,
    disk_focused: bool,
    battery_focused: bool,
) {
    let (disk_area, battery_area) = split_disk_and_battery(area, battery.is_present);
    render_disk_panel(buf, disk_area, io, theme, disk_focused);
    if let Some(battery_area) = battery_area {
        render_battery_panel(buf, battery_area, battery, theme, battery_focused);
    }
}

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
    render_disk_and_battery(
        buf,
        top[2],
        &state.snapshot.io,
        &state.snapshot.battery,
        theme,
        state.active_panel == ActivePanel::Disk,
        state.active_panel == ActivePanel::Battery,
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
    render_disk_and_battery(
        buf,
        mem_disk[1],
        &state.snapshot.io,
        &state.snapshot.battery,
        theme,
        state.active_panel == ActivePanel::Disk,
        state.active_panel == ActivePanel::Battery,
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
    render_disk_and_battery(
        buf,
        v_chunks[2],
        &state.snapshot.io,
        &state.snapshot.battery,
        theme,
        state.active_panel == ActivePanel::Disk,
        state.active_panel == ActivePanel::Battery,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_disk_and_battery_no_battery_keeps_full_area() {
        let area = Rect::new(0, 0, 30, 40);
        let (disk, battery) = split_disk_and_battery(area, false);
        assert_eq!(disk, area);
        assert!(battery.is_none());
    }

    #[test]
    fn split_disk_and_battery_too_short_skips_battery() {
        // Just under the DISK_MIN_HEIGHT + BATTERY_PANEL_HEIGHT threshold.
        let area = Rect::new(0, 0, 30, DISK_MIN_HEIGHT + BATTERY_PANEL_HEIGHT - 1);
        let (disk, battery) = split_disk_and_battery(area, true);
        assert_eq!(disk, area);
        assert!(battery.is_none());
    }

    #[test]
    fn split_disk_and_battery_carves_out_bottom_slice() {
        let area = Rect::new(5, 2, 30, 20);
        let (disk, battery) = split_disk_and_battery(area, true);
        let battery = battery.expect("battery area expected when there's enough height");

        assert_eq!(battery.height, BATTERY_PANEL_HEIGHT);
        assert_eq!(disk.height, area.height - BATTERY_PANEL_HEIGHT);
        // Battery sits directly below disk, same x/width, inside the
        // original area.
        assert_eq!(disk.y, area.y);
        assert_eq!(battery.y, disk.y + disk.height);
        assert_eq!(battery.y + battery.height, area.y + area.height);
        assert_eq!(disk.x, area.x);
        assert_eq!(battery.x, area.x);
        assert_eq!(disk.width, area.width);
        assert_eq!(battery.width, area.width);
    }
}
