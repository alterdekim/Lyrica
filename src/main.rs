use std::io;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{buffer::Buffer, layout::Rect, style::Stylize, symbols::border, text::{Line, Text}, widgets::{Block, Paragraph, Widget}, DefaultTerminal, Frame};
use tokio::sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

mod util;

#[derive(Debug)]
enum AppState {
    IPodWait,
    MainScreen,
    SoundCloud,
    Youtube,
    Preferences
}

enum AppEvent {
    SearchIPod,
    IPodFound(String),
    IPodNotFound,
}

fn initialize_async_service(sender: Sender<AppEvent>, receiver: UnboundedReceiver<AppEvent>, token: CancellationToken) {
    tokio::spawn(async move {
        let mut receiver = receiver;
        loop {
            tokio::select! {
                _ = token.cancelled() => { return; }
                r = receiver.recv() => {
                    if let Some(request) = r {
                        match request {
                            AppEvent::SearchIPod => {
                                if let Some(p) = util::search_ipod() {
                                    let _ = sender.send(AppEvent::IPodFound(p)).await;
                                } else {
                                    let _ = sender.send(AppEvent::IPodNotFound).await;
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
    });
}

#[derive(Debug)]
pub struct App {
    state: AppState,
    receiver: Receiver<AppEvent>,
    sender: UnboundedSender<AppEvent>,
    token: CancellationToken,
}

impl Default for App {
    fn default() -> Self {
        let (tx, mut rx) = mpsc::channel(1);
        let (jx, mut jr) = mpsc::unbounded_channel();
        let token = CancellationToken::new();
        initialize_async_service(tx, jr, token.clone());
        let _ = jx.send(AppEvent::SearchIPod);
        Self { state: AppState::IPodWait, receiver: rx, sender: jx, token }
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.token.is_cancelled() {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        if let Ok(event) = self.receiver.try_recv() {
            match event {
                AppEvent::IPodFound(path) => {
                    self.state = AppState::MainScreen;
                },
                AppEvent::IPodNotFound => {
                    let _ = self.sender.send(AppEvent::SearchIPod);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.code == KeyCode::Char('q') {
            self.exit();
        }
    }

    fn exit(&mut self) {
        self.token.cancel();
    }

    fn render_waiting_screen(&self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Lyrica ".bold());
        let instructions = Line::from(vec![
            " Quit ".into(),
            "<Q> ".red().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::ROUNDED);

        let counter_text = Text::from(
            vec![
                Line::from(
                    vec![
                        "Searching for iPod...".into()
                    ]
                )
            ]
        );

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.state {
            AppState::IPodWait => self.render_waiting_screen(area, buf),
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}