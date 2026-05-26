use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::clock;

use crate::app::{ActiveTab, App};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(5),
    ])
    .split(area);

    render_tabs(frame, chunks[0], app);

    match app.active_tab() {
        ActiveTab::Timer => render_timer(frame, chunks[1], app),
        ActiveTab::Statistics => render_statistics(frame, chunks[1]),
    }

    render_controls(frame, chunks[2]);
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
        Constraint::Length(2),
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

    let hint = Paragraph::new("Focus this terminal window and press a keybinding.")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[4]);
}

fn render_statistics(frame: &mut Frame, area: ratatui::layout::Rect) {
    let placeholder = Paragraph::new("Statistics coming later")
        .block(Block::default().borders(Borders::ALL).title("Statistics"))
        .alignment(Alignment::Center);
    frame.render_widget(placeholder, area);
}

fn render_controls(frame: &mut Frame, area: ratatui::layout::Rect) {
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

fn control(label: &'static str, key: &'static str) -> Span<'static> {
    Span::from(format!("{label} [{key}]")).white()
}

fn separator() -> Span<'static> {
    Span::from("  |  ").dark_gray()
}
