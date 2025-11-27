use crate::connected_users_widget::ConnectedUsersWidget;
use crate::input_widget::InputWidget;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Wrap},
};

fn calculate_scroll_to_bottom(
    messages: &[Message],
    available_height: u16,
    available_width: u16,
) -> usize {
    if messages.is_empty() {
        return 0;
    }

    let mut total_lines = 0;
    let mut messages_from_bottom = 0;

    // Work backwards from the last message to find how many fit
    for message in messages.iter().rev() {
        if message.author.is_empty() {
            continue;
        }

        let formatted_text = format!("{}: {}", message.author, message.content);
        let text_width = formatted_text.len();
        let message_lines = if text_width == 0 {
            1
        } else {
            (text_width + available_width as usize - 1) / available_width as usize
        };

        // Add spacing (except for last message)
        let lines_with_spacing = if messages_from_bottom > 0 {
            message_lines + 1
        } else {
            message_lines
        };

        // Check if adding this message exceeds available height
        if total_lines + lines_with_spacing > available_height as usize {
            break;
        }

        total_lines += lines_with_spacing;
        messages_from_bottom += 1;
    }

    // Return scroll offset to show these messages
    messages.len() - messages_from_bottom
}

fn is_near_bottom(
    messages: &[Message],
    scroll_offset: usize,
    available_height: u16,
    available_width: u16,
    threshold: usize,
) -> bool {
    if messages.is_empty() {
        return true;
    }

    // Calculate how many lines are visible from current scroll position
    let mut visible_lines = 0;
    let max_visible_lines = available_height as usize;

    for message in messages.iter().skip(scroll_offset) {
        if message.author.is_empty() {
            continue;
        }

        let formatted_text = format!("{}: {}", message.author, message.content);
        let text_width = formatted_text.len();
        let message_lines = if text_width == 0 {
            1
        } else {
            (text_width + available_width as usize - 1) / available_width as usize
        };

        // Add spacing (except for first visible message)
        let lines_with_spacing = if visible_lines > 0 {
            message_lines + 1
        } else {
            message_lines
        };

        visible_lines += lines_with_spacing;
        if visible_lines >= max_visible_lines {
            break;
        }
    }

    // Calculate how many messages are hidden below
    let messages_hidden_below = messages.len().saturating_sub(scroll_offset);
    let max_visible_messages = (max_visible_lines / 2).max(1); // Rough estimate

    // Auto-scroll if we're within threshold messages from the bottom
    messages_hidden_below <= max_visible_messages + threshold
}

pub struct Message {
    pub author: String,
    pub content: String,
}

use std::{
    io::{self, Write},
    net::TcpStream,
    sync::{Arc, Mutex, mpsc},
};

pub struct App {
    pub running: bool,
    pub input_widget: InputWidget,
    pub messages: Vec<Message>,
    pub scroll_offset: usize,
    pub should_auto_scroll: bool,
    pub username: String,
    pub server_ip: String,
    pub write_stream: Arc<Mutex<TcpStream>>,
    pub connected_users_widget: ConnectedUsersWidget,
    pub has_requested_user_list: bool,
}

pub enum Event {
    Input(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    CursorBlink,
    ServerMessage(String),
    UserListUpdate(Vec<String>),
}

impl App {
    pub fn new(username: String, server_ip: String, write_stream: Arc<Mutex<TcpStream>>) -> Self {
        Self {
            running: true,
            input_widget: InputWidget::new(username.clone()),
            messages: Vec::new(),
            scroll_offset: 0,
            should_auto_scroll: false,
            username,
            server_ip,
            write_stream,
            connected_users_widget: ConnectedUsersWidget::new(),
            has_requested_user_list: false,
        }
    }

    pub fn add_message(&mut self, author: String, content: String) {
        self.messages.push(Message { author, content });
    }

    fn scroll_down(&mut self) {
        // Don't scroll past the end of messages
        // Maximum scroll offset is when we can still see at least one message
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_up(&mut self) {
        // Don't scroll past the beginning (can't skip more messages than we have - 1)
        if self.scroll_offset < self.messages.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx: mpsc::Receiver<Event>,
        _tx: mpsc::Sender<Event>,
    ) -> io::Result<()> {
        while self.running {
            // Request user list after first iteration
            if !self.has_requested_user_list {
                self.has_requested_user_list = true;
                // Send GET_USERS command to server
                let message = "GET_USERS\n";
                let send_result = {
                    let lock_result = self.write_stream.lock();
                    match lock_result {
                        Ok(mut stream) => match stream.write_all(message.as_bytes()) {
                            Ok(_) => match stream.flush() {
                                Ok(_) => Ok(()),
                                Err(e) => Err(format!("Failed to request user list: {}", e)),
                            },
                            Err(e) => Err(format!("Failed to write to server: {}", e)),
                        },
                        Err(e) => Err(format!("Failed to lock stream: {}", e)),
                    }
                };

                if let Err(error_msg) = send_result {
                    self.add_message("System".to_string(), error_msg);
                    self.should_auto_scroll = true;
                }
            }
            match rx.recv().unwrap() {
                Event::Input(key_event) => self.handle_key_event(key_event)?,
                Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event)?,
                Event::CursorBlink => {
                    self.input_widget.update_cursor_blink();
                }
                Event::ServerMessage(message) => {
                    // Parse server message and add to messages
                    let message = message.trim().to_string();
                    if !message.is_empty() {
                        // Try to parse as "username: message" format
                        if let Some(colon_pos) = message.find(':') {
                            let author = message[..colon_pos].trim().to_string();
                            let content = message[colon_pos + 1..].trim().to_string();
                            self.add_message(author, content);
                        } else {
                            // System message (join/leave notifications)
                            self.add_message("System".to_string(), message);
                        }
                        self.should_auto_scroll = true;
                    }
                }
                Event::UserListUpdate(users) => {
                    self.connected_users_widget.set_users(users);
                }
            }

            terminal.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        const BG_PRIMARY: Color = Color::Rgb(0, 0, 0);
        const BG_SECONDARY: Color = Color::Rgb(30, 30, 30);
        const BG_SUCCESS: Color = Color::Rgb(89, 87, 86);
        const TEXT_PRIMARY: Color = Color::Rgb(255, 255, 255);
        const TEXT_SECONDARY: Color = Color::Rgb(128, 128, 128);

        let [main_area, info_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
        let [main_area, connection_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(50)]).areas(main_area);

        // Calculate input widget height
        let available_width = main_area.width.saturating_sub(4);
        let input_area_height = self.input_widget.calculate_height(available_width);
        let total_input_height = input_area_height + 3; // Input area + info area

        let [content_area, input_parent] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(total_input_height)])
                .areas(main_area);

        let [input_area_1, input_area_2] =
            Layout::vertical([Constraint::Length(input_area_height), Constraint::Length(3)])
                .areas(input_parent);

        let version_control = Line::from(Span::styled(
            " tcptalk v0.0.1 ",
            Style::default().fg(TEXT_PRIMARY),
        ))
        .centered()
        .bg(BG_SUCCESS);

        let conn_msg = format!(" Connected to {} ", self.server_ip);

        let conn_info = Line::from(Span::styled(conn_msg, Style::default().fg(TEXT_SECONDARY)))
            .bg(BG_SECONDARY);

        let [vc_area, conn_area] = Layout::horizontal([
            Constraint::Length(version_control.width() as u16),
            Constraint::Fill(1),
        ])
        .areas(info_area);

        // Create lines for messages with proper wrapping, starting from scroll offset
        let mut all_lines = Vec::new();
        let mut is_first_message = true;

        for message in self.messages.iter().skip(self.scroll_offset) {
            if !message.author.is_empty() {
                let content = format!("{}: {}", message.author, message.content);

                // Add spacing before message (except for first message)
                if !is_first_message {
                    all_lines.push(Line::from(""));
                }
                // Add message line (will wrap automatically)
                all_lines.push(Line::from(content));
                is_first_message = false;
            }
        }

        let messages_widget =
            Paragraph::new(all_lines)
                .wrap(Wrap { trim: true })
                .block(Block::new().padding(Padding {
                    left: 1,
                    right: 1,
                    top: 1,
                    bottom: 1,
                }));

        // Handle auto-scroll if flag is set
        if self.should_auto_scroll {
            let available_width = content_area.width.saturating_sub(2); // Account for padding
            let available_height = content_area.height.saturating_sub(2);

            // Only auto-scroll if user is near the bottom
            if is_near_bottom(
                &self.messages,
                self.scroll_offset,
                available_height,
                available_width,
                2,
            ) {
                self.scroll_offset =
                    calculate_scroll_to_bottom(&self.messages, available_height, available_width);
            }

            self.should_auto_scroll = false;
        }

        frame.render_widget(Block::new().bg(BG_PRIMARY), main_area);
        self.connected_users_widget.render(frame, connection_area);
        frame.render_widget(
            messages_widget,
            Rect {
                x: content_area.x,
                y: content_area.y,
                width: content_area.width,
                height: content_area.height,
            },
        );
        // Render input widget
        self.input_widget.render(frame, input_area_1, input_area_2);
        frame.render_widget(version_control, vc_area);
        frame.render_widget(conn_info, conn_area);
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> io::Result<()> {
        match mouse_event.kind {
            MouseEventKind::ScrollDown => {
                self.scroll_down();
            }
            MouseEventKind::ScrollUp => {
                self.scroll_up();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        let should_quit = self.input_widget.handle_key_event(key_event)?;
        if should_quit {
            self.running = false;
            return Ok(());
        }

        if key_event.code == KeyCode::Enter {
            // Send message to server if not empty
            if !self.input_widget.is_empty() {
                let message_content = self.input_widget.get_text();
                let message = format!("{}\n", message_content);

                // Add message to local UI immediately for better UX
                self.add_message(self.username.clone(), message_content.clone());
                self.should_auto_scroll = true;

                // Send to server in background
                let send_result = {
                    let lock_result = self.write_stream.lock();
                    match lock_result {
                        Ok(mut stream) => match stream.write_all(message.as_bytes()) {
                            Ok(_) => match stream.flush() {
                                Ok(_) => Ok(()),
                                Err(e) => Err(format!("Failed to send message: {}", e)),
                            },
                            Err(e) => Err(format!("Failed to write to server: {}", e)),
                        },
                        Err(e) => Err(format!("Failed to lock stream: {}", e)),
                    }
                };

                if let Err(error_msg) = send_result {
                    self.add_message("System".to_string(), error_msg);
                    self.should_auto_scroll = true;
                }

                // Clear input field
                self.input_widget.clear();
            }
        }

        Ok(())
    }
}
