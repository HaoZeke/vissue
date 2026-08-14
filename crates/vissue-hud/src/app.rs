//! iced chrome around [`crate::palette::Palette`]. No unit tests here.

use std::path::PathBuf;
use std::time::Duration;

use iced::event::{self, Event};
use iced::keyboard::{self, key::Named, Key};
use iced::window::{self, Mode};
use iced::{time, Element, Size, Subscription, Task};

use vissue_core::config::Layout;

use crate::attach;
use crate::palette::{Palette, PaletteKey};
use crate::summon;
use crate::theme;

const HUD_W: f32 = 640.0;
const HUD_H: f32 = 420.0;

/// First-paint inputs for the overlay.
#[derive(Clone)]
pub struct BootOpts {
    pub layout: Layout,
    pub socket: PathBuf,
    pub offline: bool,
    pub agent: String,
    pub visible: bool,
}

/// iced messages. Mapping from native keys lives here so view stays dumb.
#[derive(Debug, Clone)]
pub enum Message {
    Key(PaletteKey),
    Tick,
    WindowId(Option<window::Id>),
}

/// iced application state.
pub struct HudApp {
    pub palette: Palette,
    window_id: Option<window::Id>,
}

impl HudApp {
    fn from_palette(palette: Palette) -> Self {
        Self {
            palette,
            window_id: None,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowId(id) => {
                self.window_id = id;
                self.sync_window()
            }
            Message::Tick => {
                self.palette.poll_updates();
                if let Some(req) = summon::try_recv() {
                    let was = self.palette.visible();
                    self.palette.apply_summon(&req);
                    if was != self.palette.visible() {
                        return self.sync_window();
                    }
                }
                Task::none()
            }
            Message::Key(key) => {
                let was = self.palette.visible();
                self.palette.handle_key(key);
                if was != self.palette.visible() {
                    self.sync_window()
                } else {
                    Task::none()
                }
            }
        }
    }

    fn sync_window(&self) -> Task<Message> {
        let visible = self.palette.visible();
        let mode = if visible {
            Mode::Windowed
        } else {
            Mode::Hidden
        };
        if let Some(id) = self.window_id {
            return window::set_mode(id, mode);
        }
        window::latest().then(move |id| match id {
            Some(id) => Task::batch([
                Task::done(Message::WindowId(Some(id))),
                window::set_mode(id, mode),
            ]),
            None => Task::none(),
        })
    }
}

/// Overlay window: centered, always on top, no decorations.
pub fn overlay_window() -> window::Settings {
    window::Settings {
        size: Size::new(HUD_W, HUD_H),
        position: window::Position::Centered,
        resizable: false,
        decorations: false,
        level: window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        ..Default::default()
    }
}

/// First paint via core, attach unless `--offline`, then the iced loop.
pub fn run(opts: BootOpts) -> anyhow::Result<()> {
    let mut palette = Palette::open_core(opts.layout, opts.agent)?;
    palette.attach(&opts.socket, opts.offline, &attach::hud_hooks())?;
    if opts.visible {
        palette.show();
    } else {
        palette.hide();
    }
    crate::log::info(&format!(
        "hud start log={} status={}",
        crate::log::path().display(),
        palette.status_line()
    ));
    run_iced(palette).map_err(|err| anyhow::anyhow!("{err}"))
}

fn run_iced(palette: Palette) -> iced::Result {
    let cell = std::sync::Mutex::new(Some(palette));
    iced::application(
        move || {
            let palette = cell.lock().expect("boot").take().expect("boot once");
            boot(palette)
        },
        update,
        view,
    )
    .subscription(subscription)
    .theme(|_: &HudApp| theme::theme())
    .settings(iced::Settings {
        default_text_size: iced::Pixels(15.0),
        ..Default::default()
    })
    .window(overlay_window())
    .run()
}

fn boot(palette: Palette) -> (HudApp, Task<Message>) {
    let hidden = !palette.visible();
    let app = HudApp::from_palette(palette);
    let task = if hidden {
        app.sync_window()
    } else {
        window::latest().map(Message::WindowId)
    };
    (app, task)
}

fn update(app: &mut HudApp, message: Message) -> Task<Message> {
    app.update(message)
}

fn view(app: &HudApp) -> Element<'_, Message> {
    crate::view::view(&app.palette)
}

fn subscription(_app: &HudApp) -> Subscription<Message> {
    Subscription::batch([
        event::listen_with(|event, _status, _id| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => map_key(key),
            _ => None,
        }),
        time::every(Duration::from_millis(50)).map(|_| Message::Tick),
    ])
}

fn map_key(key: Key) -> Option<Message> {
    match key.as_ref() {
        Key::Named(Named::Enter) => Some(Message::Key(PaletteKey::Enter)),
        Key::Named(Named::Escape) => Some(Message::Key(PaletteKey::Esc)),
        Key::Named(Named::ArrowUp) => Some(Message::Key(PaletteKey::Up)),
        Key::Named(Named::ArrowDown) => Some(Message::Key(PaletteKey::Down)),
        Key::Named(Named::Backspace) => Some(Message::Key(PaletteKey::Backspace)),
        Key::Character(c) => {
            let mut chars = c.chars();
            let first = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            Some(Message::Key(PaletteKey::Char(first)))
        }
        _ => None,
    }
}
