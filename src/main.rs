use std::{collections::HashMap, error::Error, io};

use crate::theme::Theme;
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
use loading_screen::LoadingScreen;
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
mod db;
mod dlp;
mod file_system;
mod loading_screen;
mod main_screen;
mod screen;
mod sync;
mod theme;
mod util;
mod wait_screen;

#[derive(Eq, Hash, PartialEq)]
enum AppState {
    IPodWait,
    MainScreen,
    LoadingScreen,
    FileSystem,
}

pub struct App {
    state: AppState,
    screens: HashMap<AppState, Box<dyn AppScreen>>,
    receiver: Receiver<AppEvent>,
    sender: UnboundedSender<AppEvent>,
    token: CancellationToken,
    theme: Theme,
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
        screens.insert(AppState::LoadingScreen, Box::new(LoadingScreen::default()));

        Self {
            receiver: rx,
            sender: jx,
            token,
            state: AppState::IPodWait,
            screens,
            theme: Theme::default(),
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
        self.screens
            .get(&self.state)
            .unwrap()
            .render(frame, &self.theme);
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
                    AppEvent::IPodNotFound => {
                        let _ = self.sender.send(AppEvent::SearchIPod);
                    },
                    AppEvent::ITunesParsed(playlists) => {
                        let screen: &mut MainScreen = self.get_screen(&AppState::MainScreen);
                        screen.set_itunes(playlists);
                    },
                    AppEvent::SoundcloudGot(playlists) => {
                        let screen: &mut MainScreen = self.get_screen(&AppState::MainScreen);
                        screen.set_soundcloud_playlists(playlists);
                    },
                    AppEvent::OverallProgress((c, max)) => {
                        let screen: &mut LoadingScreen = self.get_screen(&AppState::LoadingScreen);
                        screen.progress = Some((c, max));
                    },
                    AppEvent::CurrentProgress(progress) => {
                        let screen: &mut LoadingScreen = self.get_screen(&AppState::LoadingScreen);
                        screen.s_progress = Some(progress);
                    },
                    AppEvent::SwitchScreen(screen) => {
                        self.state = screen;
                    }
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

    fn get_screen<T>(&mut self, state: &AppState) -> &mut T
    where
        T: 'static + AppScreen,
    {
        let a = self.screens.get_mut(state).unwrap();
        a.as_any().downcast_mut::<T>().unwrap()
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
