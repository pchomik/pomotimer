use ratatui::style::{Color, Style};
use ratatui::text::Line;
use tui_big_text::{BigText, PixelSize};

pub fn render_clock_big(time_str: &str, color: Color) -> BigText<'static> {
    BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::default().fg(color))
        .centered()
        .lines(vec![Line::from(time_str.to_string())])
        .build()
}
