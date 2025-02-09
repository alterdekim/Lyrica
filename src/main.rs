use std::{error::Error, io};

use color_eyre::Result;
use crossterm::{event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{buffer::Buffer, layout::Rect, prelude::{Backend, CrosstermBackend}, style::Stylize, symbols::border, text::{Line, Text}, widgets::{Block, Paragraph, Widget}, DefaultTerminal, Frame, Terminal};
use tokio::sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

mod util;

#[derive(Debug, Clone)]
enum AppState {
    IPodWait,
    MainScreen(String),
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
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        while !self.token.is_cancelled() {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self.state.clone(), frame.area());
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
                    self.state = AppState::MainScreen(path);
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
}

impl AppState {
    fn render_main_screen(area: Rect, buf: &mut Buffer, path: String) {
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
                        "Found iPod...".into(),
                        path.blue().bold()
                    ]
                )
            ]
        );

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }

    fn render_waiting_screen(area: Rect, buf: &mut Buffer) {
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

impl Widget for AppState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            AppState::IPodWait => AppState::render_waiting_screen(area, buf),
            AppState::MainScreen(s) => AppState::render_main_screen(area, buf, s),
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stderr = io::stderr(); // This is a special case. Normally using stdout is fine
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::default();
    app.run(&mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}