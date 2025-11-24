use crossterm::event::KeyCode;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Color, Stylize},
    widgets::Block,
};
use std::{io, sync::mpsc, thread, time::Duration};

fn main() -> io::Result<()> {
    let mut app = App { running: true };

    let mut terminal = ratatui::init();

    let (event_tx, event_rx) = mpsc::channel::<Event>();

    let tx_to_input_events = event_tx.clone();
    thread::spawn(move || {
        handle_input_events(tx_to_input_events);
    });

    let tx_to_counter_events = event_tx.clone();
    thread::spawn(move || {
        run_tick_thread(tx_to_counter_events, 30);
    });

    let app_result = app.run(&mut terminal, event_rx);

    ratatui::restore();
    app_result
}

enum Event {
    Input(crossterm::event::KeyEvent),
    Tick(u64),
}

fn handle_input_events(tx: mpsc::Sender<Event>) {
    loop {
        match crossterm::event::read().unwrap() {
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}

fn run_tick_thread(tx: mpsc::Sender<Event>, fps: u64) {
    let frame_duration = Duration::from_millis(1000 / fps);
    let mut tick: u64 = 0;
    loop {
        tx.send(Event::Tick(tick)).unwrap();
        tick = tick.wrapping_add(1);
        thread::sleep(frame_duration);
    }
}

struct App {
    running: bool,
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal, rx: mpsc::Receiver<Event>) -> io::Result<()> {
        while self.running {
            match rx.recv().unwrap() {
                Event::Input(key_event) => self.handle_key_event(key_event)?,
                Event::Tick(tick) => {
                    // if let Some(page) = self.pages.get_mut(self.selected_page) {
                    //     let _ = page.on_tick(tick);
                    // }
                }
            }

            terminal.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [vertical_area] = Layout::horizontal([Constraint::Fill(1)]).areas(frame.area());
        let [horizontal_area] = Layout::horizontal([Constraint::Fill(1)]).areas(vertical_area);

        frame.render_widget(Block::new().bg(Color::Rgb(30, 30, 30)), horizontal_area);
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        match key_event.code {
            KeyCode::Char('q') => {
                self.running = false;
            }
            _ => {}
        }

        Ok(())
    }
}
