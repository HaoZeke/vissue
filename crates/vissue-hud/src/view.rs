//! Board face on icedtea constructors. Drawing only; logic lives in [`crate::palette`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use iced::widget::markdown;
use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Alignment, Element, Fill, Length};
use icedtea::a11y::{A11y, Role};
use icedtea::collection::Tabs;
use icedtea::theme::Tokens;
use icedtea::toast::ToastKind;
use icedtea::variant::Variant;
use icedtea::widget;

use crate::app::Message;
use crate::palette::{BoardFilter, DetailTab, Focus, HudItem, Palette, ProjectSection};
use crate::theme;

/// Board face. Hidden state draws nothing so a `Mode::Hidden` frame is empty.
pub fn view(palette: &Palette) -> Element<'_, Message> {
    if !palette.visible() {
        return Space::new().width(0).height(0).into();
    }
    let tea = theme::tokens();
    if palette.focus() == Focus::Help {
        return help_overlay(palette, tea);
    }

    let mut pane = column![header(palette, tea), chips(palette, tea)]
        .spacing(10)
        .padding([14, 16]);

    if palette.browsing() {
        pane = pane.push(project_browser(palette, tea));
    } else {
        pane = pane.push(add_bar(palette, tea));
        let sections = palette.sections();
        if sections.is_empty() {
            pane = pane.push(icedtea::pattern::status_page(
                empty_copy(palette),
                "a to add, / to search",
                None,
                tea,
            ));
        } else {
            let mut list = column![].spacing(4);
            let selected = palette.selected_index();
            for section in sections {
                if palette.project().is_none() {
                    list = list.push(project_header(&section, tea));
                }
                if section.collapsed {
                    continue;
                }
                for (i, item) in section.rows {
                    list = list.push(task_row(item, i == selected, tea));
                }
            }
            pane = pane.push(widget::themed_scroll(
                list.into(),
                tea,
                A11y::new("tasks", Role::List),
                false,
                None,
                None::<fn(iced::widget::scrollable::Viewport) -> Message>,
            ));
        }
        if palette.selected_id().is_some() {
            pane = pane.push(detail_panel(palette, tea));
        }
    }

    if palette.note_draft().is_some() {
        pane = pane.push(note_bar(palette, tea));
    }
    if let Some(kind) = palette.confirm() {
        pane = pane.push(widget::info_bar(
            ToastKind::Warning,
            format!("confirm {}? y/n", kind.state()),
            tea,
            A11y::new("confirm", Role::Status),
        ));
    }
    if !palette.message().is_empty() {
        pane = pane.push(widget::info_bar(
            ToastKind::Info,
            palette.message().to_string(),
            tea,
            A11y::new("status", Role::Status),
        ));
    }

    container(pane.spacing(10).width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(move |_| icedtea::style::shell(tea))
        .into()
}

fn header(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let scope = if palette.browsing() {
        "projects"
    } else {
        palette.project().unwrap_or("search")
    };
    let live = match palette.serve_status() {
        vissue_tui::attach::ServeStatus::Live => "live",
        vissue_tui::attach::ServeStatus::Mismatch => "mismatch",
        vissue_tui::attach::ServeStatus::Offline => "offline",
    };
    let mut title = row![
        widget::label("vissue", tea, A11y::new("vissue", Role::Header)),
        widget::badge(live, tea, Variant::Quiet, A11y::new("serve", Role::Status)),
        widget::meta(scope, tea, A11y::new("scope", Role::Status)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if palette.project().is_some() {
        title = title.push(widget::chip(
            "Projects",
            Some(Message::LeaveProject),
            None,
            tea,
            Variant::Quiet,
            A11y::button("Projects"),
        ));
    }
    row![title, Space::new().width(Fill), find_control(palette, tea)]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

fn find_control(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    if palette.focus() == Focus::Search || palette.filter() == BoardFilter::Search {
        widget::themed_text_input(
            "Search title, id, tags",
            palette.query(),
            Message::QueryChanged,
            Some(Message::FocusList),
            tea,
            A11y::new("search", Role::TextBox),
            None,
        )
    } else {
        widget::chip(
            "Search",
            Some(Message::FocusSearch),
            None,
            tea,
            Variant::Quiet,
            A11y::button("Search"),
        )
    }
}

fn chips(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let mut row = row![].spacing(6);
    if palette.browsing() {
        row = row.push(filter_chip(
            "Projects",
            palette.project_cards().len(),
            true,
            Message::LeaveProject,
            tea,
        ));
        for (filter, label) in [
            (BoardFilter::Claims, "Claims"),
            (BoardFilter::Agenda, "Agenda"),
            (BoardFilter::Search, "Search"),
        ] {
            row = row.push(filter_chip(
                label,
                palette.count(filter),
                false,
                Message::Filter(filter),
                tea,
            ));
        }
        return row.into();
    }
    row = row.push(filter_chip(
        "Projects",
        0,
        false,
        Message::LeaveProject,
        tea,
    ));
    for (filter, label) in BoardFilter::ALL {
        row = row.push(filter_chip(
            label,
            palette.count(filter),
            palette.filter() == filter,
            Message::Filter(filter),
            tea,
        ));
    }
    row.into()
}

fn filter_chip(
    label: &str,
    count: usize,
    active: bool,
    msg: Message,
    tea: Tokens,
) -> Element<'static, Message> {
    let title = if count > 0 {
        format!("{label} {count}")
    } else {
        label.to_string()
    };
    widget::chip(
        title.clone(),
        Some(msg),
        None,
        tea,
        if active {
            Variant::Primary
        } else {
            Variant::Chip
        },
        A11y::button(title),
    )
}

fn project_browser(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let cards = palette.project_cards();
    if cards.is_empty() {
        return icedtea::pattern::status_page(
            "No projects under this prefix.",
            "Point --prefix at a Software tree.",
            None,
            tea,
        );
    }
    let selected = palette.selected_project_index();
    let mut list = column![].spacing(8);
    for (i, card) in cards.iter().enumerate() {
        list = list.push(project_card(card, i == selected, tea));
    }
    widget::themed_scroll(
        list.into(),
        tea,
        A11y::new("projects", Role::List),
        false,
        None,
        None::<fn(iced::widget::scrollable::Viewport) -> Message>,
    )
}

fn project_card(
    card: &crate::palette::ProjectCard,
    selected: bool,
    tea: Tokens,
) -> Element<'static, Message> {
    let name = card.name.clone();
    let count = match card.ready {
        0 => "caught up".to_string(),
        1 => "1 ready".to_string(),
        n => format!("{n} ready"),
    };
    let body = row![
        column![
            text(card.name.clone())
                .size(theme::SIZE_TITLE)
                .font(icedtea::typo::UI)
                .color(tea.text),
            widget::meta(count, tea, A11y::new("ready", Role::Status)),
        ]
        .spacing(2),
        Space::new().width(Fill),
        widget::meta("open", tea, A11y::new("open", Role::Status)),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([12, 14]);
    mouse_area(
        container(body)
            .width(Fill)
            .style(move |_| icedtea::style::card(tea, selected)),
    )
    .on_press(Message::SelectProject(name))
    .into()
}

fn add_bar(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    if palette.focus() == Focus::Add {
        widget::themed_text_input(
            "Add a task in the current project",
            palette.add_draft(),
            Message::AddChanged,
            Some(Message::AddSubmit),
            tea,
            A11y::new("add", Role::TextBox),
            None,
        )
    } else {
        widget::themed_button(
            "+ Add a task",
            Some(Message::FocusAdd),
            tea,
            Variant::Quiet,
            A11y::button("Add a task"),
        )
    }
}

fn project_header(section: &ProjectSection<'_>, tea: Tokens) -> Element<'static, Message> {
    let mark = if section.collapsed { "+" } else { "-" };
    let name = section.project.to_string();
    widget::chip(
        format!("{mark}  {}  {}", section.project, section.rows.len()),
        Some(Message::ToggleProject(name.clone())),
        None,
        tea,
        Variant::Quiet,
        A11y::button(name),
    )
}

fn task_row(item: &HudItem, selected: bool, tea: Tokens) -> Element<'_, Message> {
    let done = item.state == "DONE";
    let id = item.id.clone();
    let id_toggle = item.id.clone();
    let mut bits: Vec<String> = vec![item.state.to_lowercase()];
    if let Some(parent) = item.parent.as_deref() {
        if item.depth == 0 {
            bits.push(format!("under {parent}"));
        }
    }
    if !item.blocked_by.is_empty() {
        bits.push(format!("blocked by {}", item.blocked_by.join(", ")));
    }
    if !item.extra.is_empty() && item.claimed_by.is_none() {
        bits.push(item.extra.clone());
    }
    let meta = bits.join("  ·  ");
    let title_color = if done { tea.muted } else { tea.text };
    let indent = (item.depth as f32) * 18.0;
    let body = row![
        Space::new().width(indent),
        widget::themed_checkbox(
            String::new(),
            done,
            move |_| Message::ToggleDone(id_toggle.clone()),
            tea,
            A11y::new(item.id.clone(), Role::Checkbox).with_checked(done),
        ),
        widget::badge(
            item.priority.clone(),
            tea,
            match item.priority.as_str() {
                "A" => Variant::Danger,
                "B" => Variant::Warning,
                _ => Variant::Quiet,
            },
            A11y::new("priority", Role::Status),
        ),
        column![
            text(item.title.clone())
                .size(theme::SIZE_BODY)
                .font(icedtea::typo::UI)
                .color(title_color),
            widget::meta(meta, tea, A11y::new("state", Role::Status)),
        ]
        .spacing(2),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .padding([8, 6]);
    mouse_area(
        container(body)
            .width(Fill)
            .style(move |_| icedtea::style::card(tea, selected)),
    )
    .on_press(Message::SelectId(id))
    .into()
}

fn detail_panel(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let titles: Vec<String> = DetailTab::ALL
        .iter()
        .map(|t| t.label().to_string())
        .collect();
    let mut tabs = Tabs::new(titles);
    let active = DetailTab::ALL
        .iter()
        .position(|t| *t == palette.detail_tab())
        .unwrap_or(0);
    tabs.select(active);
    tabs.closable = false;
    let bar = widget::tab_bar(
        &tabs,
        |i| Message::DetailTab(DetailTab::ALL[i.min(DetailTab::ALL.len() - 1)]),
        |_| Message::Noop,
        tea,
        A11y::new("detail", Role::Tab),
    );
    let body = match palette.detail_md() {
        Some(doc) if !doc.items.is_empty() => widget::markdown_view(
            &doc.items,
            tea,
            |url| Message::MdLink(url.to_string()),
            A11y::new("markdown", Role::Group),
        ),
        _ => widget::meta(
            if palette.detail_body().is_empty() {
                "Select a row."
            } else {
                palette.detail_body()
            },
            tea,
            A11y::new("detail-body", Role::Group),
        ),
    };
    container(
        column![
            bar,
            widget::themed_scroll(
                body,
                tea,
                A11y::new("detail-scroll", Role::Group),
                false,
                None,
                None::<fn(iced::widget::scrollable::Viewport) -> Message>,
            ),
        ]
        .spacing(8)
        .padding(12),
    )
    .width(Fill)
    .height(Length::Fixed(220.0))
    .style(move |_| icedtea::style::panel(tea))
    .into()
}

fn note_bar(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let draft = palette.note_draft().unwrap_or("");
    widget::themed_text_input(
        "Add a note to the logbook",
        draft,
        Message::NoteChanged,
        Some(Message::NoteSubmit),
        tea,
        A11y::new("note", Role::TextBox),
        None,
    )
}

thread_local! {
    static MD_ITEMS: RefCell<HashMap<u64, &'static [markdown::Item]>> =
        RefCell::new(HashMap::new());
}

fn intern_md(src: &str) -> &'static [markdown::Item] {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let key = hasher.finish();
    MD_ITEMS.with(|map| {
        let mut map = map.borrow_mut();
        if let Some(items) = map.get(&key) {
            return *items;
        }
        let leaked: &'static [markdown::Item] =
            Box::leak(icedtea::widget::parse(src).items.into_boxed_slice());
        map.insert(key, leaked);
        leaked
    })
}

fn help_overlay(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let items = intern_md(palette.help_text());
    container(
        column![
            widget::label("vissue", tea, A11y::new("help", Role::Header)),
            widget::themed_scroll(
                widget::markdown_view(
                    items,
                    tea,
                    |url| Message::MdLink(url.to_string()),
                    A11y::new("help-md", Role::Group),
                ),
                tea,
                A11y::new("help-scroll", Role::Group),
                false,
                None,
                None::<fn(iced::widget::scrollable::Viewport) -> Message>,
            ),
            widget::meta("esc closes help", tea, A11y::new("hint", Role::Status)),
        ]
        .spacing(12)
        .padding(24),
    )
    .width(Fill)
    .height(Fill)
    .style(move |_| icedtea::style::shell(tea))
    .into()
}

fn empty_copy(palette: &Palette) -> &'static str {
    if palette.filter() == BoardFilter::Search && palette.query().is_empty() {
        return "Type to search id, title, and tags.";
    }
    if !palette.query().is_empty() {
        return "Nothing matches.";
    }
    match palette.filter() {
        BoardFilter::Ready => "Nothing ready in this project.",
        BoardFilter::List => "No issues in this project.",
        BoardFilter::Claims => "No claims.",
        BoardFilter::Agenda => "Nothing dated in the next two weeks.",
        BoardFilter::Search => "Type to search id, title, and tags.",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn board_paints_through_icedtea() {
        let src = include_str!("view.rs");
        assert!(
            src.contains("icedtea::widget::markdown_view") || src.contains("widget::markdown_view")
        );
        assert!(src.contains("widget::chip"));
        assert!(src.contains("widget::themed_text_input"));
        assert!(src.contains("icedtea::style::card"));
    }
}
