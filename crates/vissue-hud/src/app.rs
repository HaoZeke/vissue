//! iced chrome around [`crate::palette::Palette`]. No unit tests here.

use std::path::PathBuf;
use std::time::Duration;

use iced::event::{self, Event};
use iced::keyboard::{self, Key, key::Named};
use iced::window::{self, Mode};
use iced::{Element, Font, Pixels, Subscription, Task, time};

use vissue_core::config::Layout;

use crate::attach;
use crate::palette::{BoardFilter, DetailTab, Palette, PaletteKey};
use crate::summon;
use crate::theme;

const HUD_W: f32 = 960.0;
const HUD_H: f32 = 760.0;

/// First-paint inputs for the board.
#[derive(Clone, Debug)]
pub struct BootOpts {
    /// Vault root and project prefix.
    pub layout: Layout,
    /// Control socket to attach after first paint.
    pub socket: PathBuf,
    /// Skip the socket and stay on [`vissue_tui::CoreBackend`].
    pub offline: bool,
    /// Identity stamped on claims and updates.
    pub agent: String,
    /// Whether the window starts mapped.
    pub visible: bool,
}

/// iced messages. Mapping from native keys lives here so view stays dumb.
#[derive(Debug, Clone)]
pub enum Message {
    /// Board key after native mapping.
    Key(PaletteKey),
    /// 50 ms poll: serve wait plus summon socket.
    Tick,
    /// Latest iced window id, once the shell reports one.
    WindowId(Option<window::Id>),
    /// Close request: hide rather than exit.
    Close,
    /// Switch the list filter chip.
    Filter(BoardFilter),
    /// Select the row with this issue id.
    SelectId(String),
    /// Toggle DONE on this issue id.
    ToggleDone(String),
    /// Replace the search query.
    QueryChanged(String),
    /// Replace the add-task draft.
    AddChanged(String),
    /// Submit the add-task draft.
    AddSubmit,
    /// Replace the logbook note draft.
    NoteChanged(String),
    /// Submit the logbook note draft.
    NoteSubmit,
    /// Focus the add-task field.
    FocusAdd,
    /// Switch to Search and focus the query field.
    FocusSearch,
    /// Return typing to the row list.
    FocusList,
    /// Open this detail tab.
    DetailTab(DetailTab),
    /// Collapse or expand this project group.
    ToggleProject(String),
    /// Enter this project from the home list.
    SelectProject(String),
    /// Return to the home project list.
    LeaveProject,
    /// Copy a markdown link target.
    MdLink(String),
    /// Discarded click (tab-bar close, unused).
    Noop,
}

/// iced application state.
#[derive(Debug)]
pub struct HudApp {
    /// Overlay state the view reads.
    pub palette: Palette,
    window_id: Option<window::Id>,
    /// Remaining Sway IPC attempts after a show. Zero when placed or
    /// Sway is absent.
    place_tries: u8,
}

impl HudApp {
    fn from_palette(palette: Palette) -> Self {
        Self {
            palette,
            window_id: None,
            place_tries: 0,
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
                self.try_place();
                if let Some(req) = summon::try_recv() {
                    let was = self.palette.visible();
                    self.palette.apply_summon(&req);
                    if was != self.palette.visible() {
                        return self.sync_window();
                    }
                }
                Task::none()
            }
            Message::Close => {
                self.palette.hide();
                self.sync_window()
            }
            Message::Filter(filter) => {
                self.palette.set_filter(filter);
                Task::none()
            }
            Message::SelectId(id) => {
                self.palette.select_id(&id);
                Task::none()
            }
            Message::ToggleDone(id) => {
                self.palette.toggle_done(&id);
                Task::none()
            }
            Message::QueryChanged(q) => {
                self.palette.set_query(q);
                Task::none()
            }
            Message::AddChanged(text) => {
                self.palette.set_add_draft(text);
                Task::none()
            }
            Message::AddSubmit => {
                self.palette.submit_add();
                Task::none()
            }
            Message::NoteChanged(text) => {
                self.palette.set_note_draft(text);
                Task::none()
            }
            Message::NoteSubmit => {
                self.palette.submit_note();
                Task::none()
            }
            Message::FocusAdd => {
                self.palette.focus_add();
                Task::none()
            }
            Message::FocusSearch => {
                self.palette.set_filter(BoardFilter::Search);
                self.palette.focus_search();
                Task::none()
            }
            Message::FocusList => {
                self.palette.focus_list();
                Task::none()
            }
            Message::DetailTab(tab) => {
                self.palette.set_detail_tab(tab);
                Task::none()
            }
            Message::ToggleProject(project) => {
                self.palette.toggle_project(&project);
                Task::none()
            }
            Message::SelectProject(project) => {
                self.palette.enter_project(&project);
                Task::none()
            }
            Message::LeaveProject => {
                self.palette.leave_project();
                Task::none()
            }
            Message::MdLink(url) => {
                self.palette.set_clipboard(url);
                Task::none()
            }
            Message::Noop => Task::none(),
            Message::Key(key) => {
                let was = self.palette.visible();
                let before = self.palette.clipboard().to_string();
                self.palette.handle_key(key);
                let clip = self.palette.clipboard().to_string();
                let mut tasks = Vec::new();
                if clip != before && !clip.is_empty() {
                    tasks.push(iced::clipboard::write(clip));
                }
                if was != self.palette.visible() {
                    tasks.push(self.sync_window());
                }
                Task::batch(tasks)
            }
        }
    }

    fn sync_window(&mut self) -> Task<Message> {
        let visible = self.palette.visible();
        if visible {
            self.place_tries = 20;
        } else {
            self.place_tries = 0;
        }
        let mode = if visible {
            Mode::Windowed
        } else {
            Mode::Hidden
        };
        if let Some(id) = self.window_id {
            self.try_place();
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

    fn try_place(&mut self) {
        if self.place_tries == 0 {
            return;
        }
        if crate::place::place_overlay() || !crate::place::sway_available() {
            self.place_tries = 0;
        } else {
            self.place_tries = self.place_tries.saturating_sub(1);
        }
    }
}

/// Undecorated always-on-top overlay. Sway is told to float it over IPC.
pub fn board_window() -> window::Settings {
    let boot = icedtea::app::Boot::new("vissue", crate::place::OVERLAY_APP_ID)
        .overlay()
        .size(HUD_W, HUD_H)
        .min_size(360.0, 420.0);
    icedtea::app::bootstrap(&boot).window
}

/// First paint via core, attach unless `--offline`, then the iced loop.
///
/// # Errors
///
/// Returns an error if the vault cannot be opened, attach reload fails, or
/// the iced application cannot start.
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
    .title(|_: &HudApp| "vissue".to_string())
    .default_font(icedtea::typo::UI)
    .settings(iced::Settings {
        default_text_size: Pixels::from(icedtea::typo::BODY),
        default_font: icedtea::typo::UI,
        ..Default::default()
    })
    .window(board_window())
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
        event::listen_with(|event, status, _id| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if status == event::Status::Captured && !is_nav_key(&key) {
                    return None;
                }
                map_key(key)
            }
            Event::Window(window::Event::CloseRequested) => Some(Message::Close),
            _ => None,
        }),
        time::every(Duration::from_millis(50)).map(|_| Message::Tick),
    ])
}

fn is_nav_key(key: &Key) -> bool {
    matches!(
        key,
        Key::Named(Named::Escape | Named::Enter | Named::ArrowUp | Named::ArrowDown | Named::Tab)
    )
}

fn map_key(key: Key) -> Option<Message> {
    match key.as_ref() {
        Key::Named(Named::Enter) => Some(Message::Key(PaletteKey::Enter)),
        Key::Named(Named::Escape) => Some(Message::Key(PaletteKey::Esc)),
        Key::Named(Named::ArrowUp) => Some(Message::Key(PaletteKey::Up)),
        Key::Named(Named::ArrowDown) => Some(Message::Key(PaletteKey::Down)),
        Key::Named(Named::Backspace) => Some(Message::Key(PaletteKey::Backspace)),
        Key::Named(Named::Space) => Some(Message::Key(PaletteKey::Space)),
        Key::Named(Named::Tab) => Some(Message::Key(PaletteKey::Tab)),
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

// Keep Font in scope so a missing FACE constant fails here, not in view.
const _: Font = theme::FACE;
