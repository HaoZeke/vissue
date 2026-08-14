//! Key dispatch. Bindings are listed on `?`.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// What the event loop does after a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Continue,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Ready,
    List,
    Claims,
    Agenda,
    Search,
}

impl Pane {
    pub const ALL: [Pane; 5] = [
        Pane::Ready,
        Pane::List,
        Pane::Claims,
        Pane::Agenda,
        Pane::Search,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::List => "List",
            Self::Claims => "Claims",
            Self::Agenda => "Agenda",
            Self::Search => "Search",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Self {
        Self::ALL[i % Self::ALL.len()]
    }

    pub fn next(self) -> Self {
        Self::from_index(self.index() + 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Show,
    Excerpt,
    Tree,
    Related,
}

impl DetailTab {
    pub const ALL: [DetailTab; 4] = [
        DetailTab::Show,
        DetailTab::Excerpt,
        DetailTab::Tree,
        DetailTab::Related,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Excerpt => "excerpt",
            Self::Tree => "tree",
            Self::Related => "related",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Rows,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Search,
    Note,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmKind {
    Done,
    Cancelled,
}

impl ConfirmKind {
    pub fn state(self) -> &'static str {
        match self {
            Self::Done => "DONE",
            Self::Cancelled => "CANCELLED",
        }
    }
}

pub fn is_press(key: KeyEvent) -> bool {
    key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat
}

pub fn char_of(key: KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            Some(c)
        }
        _ => None,
    }
}

pub const HELP: &str = "\
vissue tui

j/k, arrows   move
Tab, 1-5      pane (Ready List Claims Agenda Search)
Enter         focus detail / cycle detail tab
p             project filter
/             search
c             claim
n             note
s             cycle TODO / STARTED / BLOCKED
D             DONE (confirm)
X             CANCELLED (confirm)
o             open (shared selection)
y             copy id
R             reload
?             this help
q / Esc       quit / back

Body edits stay in the file.
body lives in file; open the range above
";
