use std::{any::Any, cell::RefCell, collections::HashMap, error::Error, io, ops::Deref, path::{Path, PathBuf}};

use color_eyre::Result;
use crossterm::{event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{buffer::Buffer, layout::{Layout, Rect}, prelude::{Backend, CrosstermBackend}, style::{Color, Stylize}, symbols::border, text::{Line, Text}, widgets::{Block, Paragraph, Tabs, Widget}, DefaultTerminal, Frame, Terminal};
use main_screen::MainScreen;
use screen::AppScreen;
use sync::AppEvent;
use tokio::{fs::File, io::AsyncReadExt, sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender}};
use tokio_util::sync::CancellationToken;
use ratatui::prelude::Constraint::{Length, Min};
use wait_screen::WaitScreen;

mod dlp;
mod util;
mod config;
mod screen;
mod main_screen;
mod wait_screen;
mod sync;

#[derive(Eq, Hash, PartialEq)]
enum AppState {
    IPodWait,
    MainScreen
}

pub struct App {
    state: AppState,
    screens: HashMap<AppState, Box<dyn AppScreen>>,
    receiver: Receiver<AppEvent>,
    sender: UnboundedSender<AppEvent>,
    token: CancellationToken,
}

impl Default for App {
    fn default() -> Self {
        let (tx, mut rx) = mpsc::channel(1);
        let (jx, mut jr) = mpsc::unbounded_channel();
        let token = CancellationToken::new();

        sync::initialize_async_service(tx, jr, token.clone());
        
        let _ = jx.send(AppEvent::SearchIPod);
        
        let mut screens: HashMap<AppState, Box<dyn AppScreen>> = HashMap::new();
        screens.insert(AppState::IPodWait, Box::new(WaitScreen::default()));
        screens.insert(AppState::MainScreen, Box::new(MainScreen::new()));

        Self { receiver: rx, sender: jx, token, state: AppState::IPodWait, screens }
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

    fn draw(&mut self, frame: &mut Frame) {
        self.screens.get(&self.state).unwrap().render(frame);
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
                    let _ = self.sender.send(AppEvent::ParseItunes(path));
                },
                AppEvent::IPodNotFound => {
                    let _ = self.sender.send(AppEvent::SearchIPod);
                },
                AppEvent::ITunesParsed(xdb) => {

                },
                AppEvent::SoundcloudGot(playlists) => {
                    let a = self.screens.get_mut(&AppState::MainScreen).unwrap();
                    let screen: &mut MainScreen = match a.as_any().downcast_mut::<MainScreen>() {
                        Some(b) => b,
                        None => panic!("&a isn't a B!"),
                    };
                    screen.soundcloud = Some(playlists);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        self.screens.get_mut(&self.state).unwrap().handle_key_event(key_event);
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.token.cancel();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stderr = io::stdout();
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