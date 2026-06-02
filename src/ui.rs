use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph},
};

use crate::{app::StatsScale, clock};

use crate::app::{ActiveTab, App};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Recalculate bar count when on the statistics tab, so resizes are reflected.
    if app.active_tab() == ActiveTab::Statistics {
        let inner_width = area.width.saturating_sub(2 * STATS_H_MARGIN + 2);
        let min_slot = MIN_BAR_WIDTH + BAR_GAP;
        let max_bars = (inner_width / min_slot).max(1);
        let bar_count = (1..=max_bars)
            .rev()
            .find(|n| inner_width.is_multiple_of(*n) && inner_width / n >= min_slot)
            .unwrap_or(max_bars) as u32;
        app.set_stats_bar_count(bar_count);
    }
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(5),
    ])
    .split(area);

    render_tabs(frame, chunks[0], app);

    match app.active_tab() {
        ActiveTab::Timer => {
            render_timer(frame, chunks[1], app);
            render_timer_controls(frame, chunks[2]);
        }
        ActiveTab::Statistics => {
            render_statistics(frame, chunks[1], app);
            render_stats_controls(frame, chunks[2]);
        }
    }
}

fn render_tabs(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let tabs = ["Timer", "Statistics"];
    let num_tabs = tabs.len();
    let tab_width = area.width / num_tabs as u16;

    for (i, label) in tabs.iter().enumerate() {
        let x = area.x + i as u16 * tab_width;
        let tab_area = ratatui::layout::Rect {
            x,
            y: area.y,
            width: tab_width,
            height: area.height,
        };

        let style = if i == app.active_tab_index() {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let paragraph = Paragraph::new(*label)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(style);
        frame.render_widget(paragraph, tab_area);
    }
}

fn render_timer(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(2),
        Constraint::Length(8),
        Constraint::Min(0),
        // Constraint::Length(2),
    ])
    .split(area);

    let status = if app.is_running() {
        "Running"
    } else {
        "Paused"
    };
    let phase = Paragraph::new(format!("{} - {}", app.phase().label(), status))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(phase, chunks[1]);

    let clock_widget = clock::render_clock_big(&app.clock_text(), Color::Green);
    frame.render_widget(clock_widget, chunks[2]);

    // let hint = Paragraph::new("Focus this terminal window and press a keybinding.")
    //     .alignment(Alignment::Center)
    //     .style(Style::default().fg(Color::DarkGray));
    // frame.render_widget(hint, chunks[4]);
}

/// Left and right margin between the chart block and the terminal edge (in columns).
const STATS_H_MARGIN: u16 = 4;
/// Gap between bars in the chart (in columns).
const BAR_GAP: u16 = 1;
/// Minimum bar width (in columns). Bars won't be narrower than this.
const MIN_BAR_WIDTH: u16 = 4;

fn render_statistics(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let vchunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);
    let main = vchunks[1];

    // Add horizontal margin on both sides.
    let hchunks = Layout::horizontal([
        Constraint::Length(STATS_H_MARGIN),
        Constraint::Min(0),
        Constraint::Length(STATS_H_MARGIN),
    ])
    .split(main);
    let center = hchunks[1];

    // Available inner width after block borders (│). ratatui uses:
    //   total = N * (bar_width + bar_gap)
    // Pick the largest N where the slot (bar_width + bar_gap) divides inner_width
    // evenly and bar_width >= MIN_BAR_WIDTH.
    let inner_width = center.width.saturating_sub(2);
    let min_slot = MIN_BAR_WIDTH + BAR_GAP;
    let max_bars = (inner_width / min_slot).max(1);
    let bar_count = (1..=max_bars)
        .rev()
        .find(|n| inner_width.is_multiple_of(*n) && inner_width / n >= min_slot)
        .unwrap_or(max_bars) as u32;

    match app.stats_scale() {
        StatsScale::Week => render_daily_chart(frame, center, app, bar_count),
        StatsScale::Month => render_monthly_chart(frame, center, app, bar_count),
    }
}

fn render_daily_chart(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, bar_count: u32) {
    let (start, end) = app.daily_window_range(bar_count);
    let header = Paragraph::new(format!(
        "Daily Focus Time — {} to {}",
        start.format("%b %d"),
        end.format("%b %d"),
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::Cyan));

    let inner = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    frame.render_widget(header, inner[0]);

    let data = app.daily_data();
    if data.iter().all(|d| d.total_seconds == 0) {
        let hint = Paragraph::new("No data for this period.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, inner[2]);
        return;
    }

    let max_minutes = data
        .iter()
        .map(|d| d.total_seconds / 60)
        .max()
        .unwrap_or(1)
        .max(1);

    let bars: Vec<Bar<'_>> = data
        .iter()
        .map(|d| {
            Bar::default()
                .value(d.total_seconds / 60)
                .label(Line::from(d.date.format("%a").to_string()))
                .style(Style::default().fg(Color::Green))
        })
        .collect();

    // ratatui's group_ticks uses: total = N*(bar_width + bar_gap).
    // Solve for bar_width to fill the inner area exactly.
    let available_width = inner[2].width;
    let bar_width = (available_width / bars.len().max(1) as u16)
        .saturating_sub(BAR_GAP)
        .max(1);

    let summary = Paragraph::new(format!("Focus Time (minutes) — max ~{max_minutes}m"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    frame.render_widget(summary, inner[1]);

    let chart = BarChart::default()
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(BAR_GAP);

    frame.render_widget(chart, inner[3]);
}

fn render_monthly_chart(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, bar_count: u32) {
    let (start, end) = app.monthly_window_range(bar_count);
    let header = Paragraph::new(format!(
        "Monthly Focus Time — {} to {}",
        start.format("%b %Y"),
        end.format("%b %Y"),
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::Cyan));

    let inner = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    frame.render_widget(header, inner[0]);

    let data = app.monthly_data();
    if data.iter().all(|d| d.total_seconds == 0) {
        let hint = Paragraph::new("No data for this period.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, inner[2]);
        return;
    }

    let max_hours = data
        .iter()
        .map(|d| d.total_seconds / 3600)
        .max()
        .unwrap_or(1)
        .max(1);

    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let bars: Vec<Bar<'_>> = data
        .iter()
        .map(|d| {
            let label = MONTHS.get(d.month as usize - 1).copied().unwrap_or("???");
            Bar::default()
                .value(d.total_seconds / 3600)
                .label(Line::from(label))
                .style(Style::default().fg(Color::Blue))
        })
        .collect();

    // ratatui's group_ticks uses: total = N*(bar_width + bar_gap).
    // Solve for bar_width to fill the inner area exactly.
    let available_width = inner[2].width;
    let bar_width = (available_width / bars.len().max(1) as u16)
        .saturating_sub(BAR_GAP)
        .max(1);

    let summary = Paragraph::new(format!("Focus Time (hours) — max ~{max_hours}h"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Blue));
    frame.render_widget(summary, inner[1]);

    let chart = BarChart::default()
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(BAR_GAP);

    frame.render_widget(chart, inner[3]);
}

fn render_timer_controls(frame: &mut Frame, area: ratatui::layout::Rect) {
    let controls = Line::from(vec![
        control("Change Tab", "t"),
        separator(),
        control("Start/Pause", "space"),
        separator(),
        control("Stop", "s"),
        separator(),
        control("Reset", "r"),
        separator(),
        control("Next", "n"),
        separator(),
        control("Quit", "q"),
    ]);

    let block = Block::default().borders(Borders::ALL);
    let [_, mid, _] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(block.inner(area));

    let paragraph = Paragraph::new(controls).alignment(Alignment::Center);
    frame.render_widget(block, area);
    frame.render_widget(paragraph, mid);
}

fn render_stats_controls(frame: &mut Frame, area: ratatui::layout::Rect) {
    let controls = Line::from(vec![
        control("Change Tab", "t"),
        separator(),
        control("Change Scale", "s"),
        separator(),
        control("Next", "n"),
        separator(),
        control("Previous", "p"),
        separator(),
        control("Reset", "r"),
        separator(),
        control("Quit", "q"),
    ]);

    let block = Block::default().borders(Borders::ALL);
    let [_, mid, _] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(block.inner(area));

    let paragraph = Paragraph::new(controls).alignment(Alignment::Center);
    frame.render_widget(block, area);
    frame.render_widget(paragraph, mid);
}

fn control(label: &'static str, key: &'static str) -> Span<'static> {
    Span::from(format!("{label} [{key}]")).white()
}

fn separator() -> Span<'static> {
    Span::from("  |  ").dark_gray()
}
