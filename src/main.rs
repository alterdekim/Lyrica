use std::{collections::HashMap, error::Error, io};

use color_eyre::Result;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use main_screen::MainScreen;
use ratatui::{
    prelude::{Backend, CrosstermBackend},
    widgets::Widget,
    Frame, Terminal,
};
use screen::AppScreen;
use sync::AppEvent;
use tokio::sync::mpsc::{self, Receiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use wait_screen::WaitScreen;

mod config;
mod dlp;
mod main_screen;
mod screen;
mod sync;
mod util;
mod wait_screen;

#[derive(Eq, Hash, PartialEq)]
enum AppState {
    IPodWait,
    MainScreen,
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
        let (tx, rx) = mpsc::channel(10);
        let (jx, jr) = mpsc::unbounded_channel();
        let token = CancellationToken::new();

        sync::initialize_async_service(tx, jr, token.clone());

        let _ = jx.send(AppEvent::SearchIPod);

        let mut screens: HashMap<AppState, Box<dyn AppScreen>> = HashMap::new();
        screens.insert(AppState::IPodWait, Box::new(WaitScreen::default()));
        screens.insert(AppState::MainScreen, Box::new(MainScreen::new(jx.clone())));

        Self {
            receiver: rx,
            sender: jx,
            token,
            state: AppState::IPodWait,
            screens,
        }
    }
}

impl App {
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut reader = EventStream::new();
        while !self.token.is_cancelled() {
            let _ = self.handle_events(&mut reader).await;
            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        self.screens.get(&self.state).unwrap().render(frame);
    }

    async fn handle_events(&mut self, reader: &mut EventStream) {
        tokio::select! {
            Some(Ok(event)) = reader.next() => {
                match event {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_key_event(key_event);
                    }
                    _ => {}
                }
            },
            Some(event) = self.receiver.recv() => {
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
                        let screen: &mut MainScreen = a.as_any().downcast_mut::<MainScreen>().unwrap();
                        screen.set_soundcloud_playlists(playlists);
                    },
                    AppEvent::OverallProgress((c, max)) => {
                        let a = self.screens.get_mut(&AppState::MainScreen).unwrap();
                        let screen: &mut MainScreen = a.as_any().downcast_mut::<MainScreen>().unwrap();
                        screen.progress = Some((c, max));
                        screen.download_screen();
                    },
                    AppEvent::CurrentProgress(progress) => {
                        let a = self.screens.get_mut(&AppState::MainScreen).unwrap();
                        let screen: &mut MainScreen = a.as_any().downcast_mut::<MainScreen>().unwrap();
                        screen.s_progress = Some(progress);
                        screen.download_screen();
                    },
                    _ => {}
                }
            }
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        self.screens
            .get_mut(&self.state)
            .unwrap()
            .handle_key_event(key_event);
        if let KeyCode::Char('q') = key_event.code {
            self.exit()
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
    let _ = app.run(&mut terminal).await;

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
