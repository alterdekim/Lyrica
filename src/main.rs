use std::{error::Error, io, path::{Path, PathBuf}};

use color_eyre::Result;
use config::LyricaConfiguration;
use crossterm::{event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{buffer::Buffer, layout::{Layout, Rect}, prelude::{Backend, CrosstermBackend}, style::{Color, Stylize}, symbols::border, text::{Line, Text}, widgets::{Block, Paragraph, Tabs, Widget}, DefaultTerminal, Frame, Terminal};
use screen::MainScreen;
use soundcloud::sobjects::CloudPlaylists;
use strum::IntoEnumIterator;
use tokio::{fs::File, io::AsyncReadExt, sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender}};
use tokio_util::sync::CancellationToken;
use itunesdb::xobjects::XDatabase;
use ratatui::prelude::Constraint::{Length, Min};

mod util;
mod config;
mod tabs;
mod screen;

fn get_configs_dir() -> PathBuf {
    let mut p = dirs::home_dir().unwrap();
    p.push(".lyrica");
    p
}

#[derive(Debug, Clone)]
enum AppState {
    IPodWait,
    MainScreen(crate::screen::MainScreen)
}

enum AppEvent {
    SearchIPod,
    IPodFound(String),
    IPodNotFound,
    ParseItunes(String),
    ITunesParsed(XDatabase),
    SoundcloudGot(CloudPlaylists)
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
                                /*if let Some(p) = util::search_ipod() {
                                    let _ = sender.send(AppEvent::IPodFound(p)).await;
                                } else {
                                    let _ = sender.send(AppEvent::IPodNotFound).await;
                                }*/
                                let _ = sender.send(AppEvent::IPodFound("D:\\Documents\\RustroverProjects\\itunesdb\\ITunesDB\\two_tracks".to_string())).await;
                            },
                            AppEvent::ParseItunes(path) => {
                                // todo: parse itunes
                                let _ = std::fs::create_dir_all(get_configs_dir());
                                let mut cd = get_configs_dir();
                                cd.push("idb");
                                let mut p: PathBuf = Path::new(&path).into();
                               // p.push("iPod_Control");
                             //   p.push("iTunes");
                              //  p.set_file_name("iTunesDB");
                                let _ = std::fs::copy(p, &cd);
                                let mut file = File::open(cd).await.unwrap();
                                let mut contents = vec![];
                                file.read_to_end(&mut contents).await.unwrap();
                                let xdb = itunesdb::deserializer::parse_bytes(&contents);
                                let _ = sender.send(AppEvent::ITunesParsed(xdb)).await;

                                let mut p = get_configs_dir();
                                p.push("config");
                                p.set_extension(".toml");
                                if !p.exists() { return; }
                                let mut file = File::open(p).await.unwrap();
                                let mut content = String::new();
                                file.read_to_string(&mut content).await.unwrap();
                                let config: LyricaConfiguration = toml::from_str(&content).unwrap();
                                
                                let app_version = soundcloud::get_app().await.unwrap().unwrap();
                                let client_id = soundcloud::get_client_id().await.unwrap().unwrap();
                                let playlists = soundcloud::get_playlists(config.get_soundcloud().user_id, client_id, app_version).await.unwrap();

                                let _ = sender.send(AppEvent::SoundcloudGot(playlists)).await;
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
                    self.state = AppState::MainScreen(MainScreen::new());
                    let _ = self.sender.send(AppEvent::ParseItunes(path));
                },
                AppEvent::IPodNotFound => {
                    let _ = self.sender.send(AppEvent::SearchIPod);
                },
                AppEvent::ITunesParsed(xdb) => {

                },
                AppEvent::SoundcloudGot(playlists) => {
                    if let AppState::MainScreen(screen) = &self.state {
                        let mut screen = screen.clone();
                        screen.soundcloud = Some(playlists);
                        self.state = AppState::MainScreen(screen);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if let AppState::MainScreen(screen) = &self.state {
            let mut screen = screen.clone();
            screen.handle_key_event(key_event);
            self.state = AppState::MainScreen(screen);
        }
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.token.cancel();
    }
}

impl AppState {
    fn render_main_screen(area: Rect, buf: &mut Buffer, screen: &mut MainScreen) {
        let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0), Length(7)]);
        let [tabs_area, title_area] = horizontal.areas(header_area);

        MainScreen::render_title(title_area, buf);
        screen.render_tabs(tabs_area, buf);
        screen.selected_tab.render(inner_area, buf);
        MainScreen::render_footer(footer_area, buf);
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
            AppState::MainScreen(mut s) => AppState::render_main_screen(area, buf, &mut s),
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