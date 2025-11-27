use ratatui::{
    Frame,
    layout::Rect,
    prelude::Stylize,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
};

pub struct ConnectedUsersWidget {
    pub users: Vec<String>,
}

impl ConnectedUsersWidget {
    pub fn new() -> Self {
        Self { users: Vec::new() }
    }

    pub fn set_users(&mut self, users: Vec<String>) {
        self.users = users;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        const BG_PRIMARY: Color = Color::Rgb(0, 0, 0);
        const TEXT_PRIMARY: Color = Color::Rgb(255, 255, 255);

        let mut lines = vec![Line::from(Span::styled(
            format!("List of current connections ({})", self.users.len()),
            Style::default().bold(),
        ))];

        for user in &self.users {
            lines.push(Line::from(format!("[o] {}", user)));
        }

        let widget = Paragraph::new(lines)
            .style(Style::default().fg(TEXT_PRIMARY))
            .bg(BG_PRIMARY)
            .block(Block::new().padding(Padding {
                left: 0,
                right: 0,
                top: 1,
                bottom: 1,
            }));

        frame.render_widget(widget, area);
    }
}
