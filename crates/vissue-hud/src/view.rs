//! Board face on icedtea constructors. Drawing only; logic lives in [`crate::palette`].

use iced::widget::{Space, column, container, mouse_area, row, text};
use iced::{Alignment, Element, Fill, Length};
use icedtea::a11y::{A11y, Role};
use icedtea::collection::Tabs;
use icedtea::i18n::Direction;
use icedtea::icon::Icons;
use icedtea::theme::Tokens;
use icedtea::toast::ToastKind;
use icedtea::typo::FontFace;
use icedtea::variant::Variant;
use icedtea::widget;

use crate::app::Message;
use crate::palette::{BoardFilter, DetailTab, Focus, HudItem, Palette, TreeRow};
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

    let mut pane = column![header(palette, tea), actions(palette, tea)]
        .spacing(10)
        .padding([14, 16]);

    if palette.browsing() {
        pane = pane.push(project_browser(palette, tea));
    } else {
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
            let group = palette.project().is_none();
            for section in &sections {
                let mut rows = column![].spacing(4);
                if !section.collapsed || group {
                    for (i, item) in &section.rows {
                        rows = rows.push(task_row(item, *i == selected, tea));
                    }
                }
                if group {
                    list = list.push(project_group(
                        section.project,
                        section.rows.len(),
                        !section.collapsed,
                        !palette.query().is_empty(),
                        rows,
                        tea,
                    ));
                } else {
                    list = list.push(rows);
                }
            }
            let list = pane_scroll(list.into(), tea, A11y::new("tasks", Role::List));
            if palette.selected_id().is_some() {
                pane = pane.push(list_detail(palette, list, tea));
            } else {
                pane = pane.push(list);
            }
        }
        if sections.is_empty() && palette.selected_id().is_some() {
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
    let (live, live_var) = match palette.serve_status() {
        vissue_tui::attach::ServeStatus::Live => ("live", Variant::Success),
        vissue_tui::attach::ServeStatus::Mismatch => ("mismatch", Variant::Warning),
        vissue_tui::attach::ServeStatus::Offline => ("offline", Variant::Quiet),
    };
    row![
        widget::label("vissue", tea, A11y::new("vissue", Role::Header)),
        widget::badge(
            live,
            None,
            tea,
            live_var,
            widget::BadgeSize::Small,
            A11y::new("serve", Role::Status),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn actions(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    row![
        chips(palette, tea),
        container(find_control(palette, tea)).width(Fill),
        add_bar(palette, tea),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn find_control(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    widget::search_input_clear(
        palette.query(),
        Message::QueryChanged,
        Some(Message::QueryChanged(String::new())),
        Some(Message::FocusList),
        tea,
        A11y::new("Search title, id, tags", Role::TextBox),
        None,
    )
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
    for (filter, label) in BoardFilter::CHIPS {
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
        widget::ChipKind::Filter,
        Icons::NONE,
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
    let muted = tea.muted;
    widget::list_view(
        palette.project_list(),
        palette.project_selection(),
        |click| Message::PickProject(click.id),
        tea,
        palette.project_window(),
        56.0,
        2,
        Message::ProjectScroll,
        "No projects under this prefix.",
        move |_| muted,
        None,
        icedtea::collection::RowFace::Card {
            meter: None::<fn(usize) -> f32>,
        },
        |_| Message::Noop,
        A11y::new("projects", Role::List),
    )
}

fn add_bar(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    if palette.focus() == Focus::Add {
        widget::themed_text_input(
            "Add a task in the current project",
            palette.add_draft(),
            Message::AddChanged,
            Some(Message::AddSubmit),
            widget::FieldOpts::NONE,
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
            Icons::NONE,
            A11y::button("Add a task"),
        )
    }
}

fn project_group<'a>(
    project: &str,
    count: usize,
    open: bool,
    searching: bool,
    rows: iced::widget::Column<'a, Message>,
    tea: Tokens,
) -> Element<'a, Message> {
    let name = project.to_string();
    widget::expander(
        project.to_string(),
        Some(badge_mark(
            count.to_string(),
            if searching {
                Variant::Primary
            } else {
                Variant::Chip
            },
            tea,
            "count",
        )),
        rows.into(),
        widget::Peek::Pixels(0.0),
        open,
        if open { 1.0 } else { 0.0 },
        move |_| Message::ToggleProject(name.clone()),
        tea,
        A11y::new(project.to_string(), Role::Group),
    )
}

fn tree_row<'a>(
    palette: &'a Palette,
    node: TreeRow<'a>,
    selected: bool,
    tea: Tokens,
) -> Element<'a, Message> {
    let twisty: Element<'a, Message> = if node.has_children {
        let mark = if node.expanded { "▾" } else { "▸" };
        let id = node.tea_id;
        mouse_area(text(mark).size(tea.body()).color(tea.muted))
            .on_press(Message::TreeToggle(id))
            .into()
    } else {
        Space::new().width(14).into()
    };
    let item = palette.item_by_id(node.issue_id);
    let marks = issue_marks(
        item.map(|i| i.priority.as_str()),
        node.state,
        node.blocked,
        item.is_some_and(|i| i.claimed_by.is_some()),
        tea,
    );
    let title_color = if node.state == "DONE" {
        tea.muted
    } else {
        tea.text
    };
    let pick = node.tea_id;
    let open = node.issue_id.to_string();
    let mut line = row![
        Space::new().width((node.depth as f32) * 16.0),
        twisty,
        marks,
        text(node.title.to_string())
            .size(tea.body())
            .font(icedtea::typo::UI)
            .color(title_color)
            .wrapping(iced::widget::text::Wrapping::Word)
            .width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Fill);
    if let Some(item) = item {
        line = line.push(project_mark(&item.project, tea));
    }
    let body = line.padding([4, 4]);
    mouse_area(
        container(body)
            .width(Fill)
            .style(move |_| icedtea::style::card(tea, selected)),
    )
    .on_press(Message::TreePick(pick))
    .on_double_click(Message::OpenIssue(open))
    .into()
}

fn task_row(item: &HudItem, selected: bool, tea: Tokens) -> Element<'_, Message> {
    let done = item.state == "DONE";
    let id = item.id.clone();
    let id_toggle = item.id.clone();
    let marks = issue_marks(
        Some(item.priority.as_str()),
        &item.state,
        !item.blocked_by.is_empty(),
        item.claimed_by.is_some(),
        tea,
    );
    let mut bits: Vec<String> = Vec::new();
    if let Some(parent) = item.parent.as_deref()
        && item.depth == 0
    {
        bits.push(format!("under {parent}"));
    }
    if !item.extra.is_empty() && item.claimed_by.is_none() {
        bits.push(item.extra.clone());
    }
    if let Some(due) = item.due.as_deref() {
        bits.push(due.to_string());
    }
    let title_color = if done { tea.muted } else { tea.text };
    let indent = (item.depth as f32) * 18.0;
    let line = row![
        marks,
        text(item.title.clone())
            .size(tea.body())
            .font(icedtea::typo::UI)
            .color(title_color),
        project_mark(&item.project, tea),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Fill);
    let mut text_col = column![line]
        .spacing(4)
        .width(Fill)
        .align_x(Alignment::Start);
    if !bits.is_empty() {
        text_col = text_col.push(widget::meta(
            bits.join("  ·  "),
            tea,
            A11y::new("state", Role::Status),
        ));
    }
    let box_el = widget::themed_checkbox(
        "",
        done,
        move |_| Message::ToggleDone(id_toggle.clone()),
        tea,
        A11y::new("done", Role::Checkbox),
    );
    let body = row![Space::new().width(indent), box_el, text_col,]
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Fill)
        .padding([8, 6]);
    mouse_area(
        container(body)
            .width(Fill)
            .style(move |_| icedtea::style::card(tea, selected)),
    )
    .on_press(Message::SelectId(id))
    .into()
}

fn issue_marks(
    priority: Option<&str>,
    state: &str,
    blocked: bool,
    claimed: bool,
    tea: Tokens,
) -> Element<'static, Message> {
    let mut marks = row![].spacing(4).align_y(Alignment::Center);
    if let Some(priority) = priority {
        marks = marks.push(badge_priority(priority, tea));
    }
    marks = marks.push(badge_state(state, tea));
    if blocked {
        marks = marks.push(badge_mark("blocked", Variant::Warning, tea, "blocked"));
    }
    if claimed {
        marks = marks.push(badge_mark("claimed", Variant::Primary, tea, "claimed"));
    }
    marks.into()
}

fn project_mark(project: &str, tea: Tokens) -> Element<'static, Message> {
    badge_mark(project.to_string(), Variant::Chip, tea, "project")
}

fn badge_priority(priority: &str, tea: Tokens) -> Element<'static, Message> {
    badge_mark(
        priority.to_string(),
        match priority {
            "A" => Variant::Danger,
            "B" => Variant::Warning,
            _ => Variant::Quiet,
        },
        tea,
        "priority",
    )
}

fn badge_state(state: &str, tea: Tokens) -> Element<'static, Message> {
    badge_mark(
        state.to_lowercase(),
        match state {
            "STARTED" => Variant::Primary,
            "BLOCKED" => Variant::Warning,
            "DONE" => Variant::Success,
            "CANCELLED" => Variant::Quiet,
            _ => Variant::Chip,
        },
        tea,
        "state",
    )
}

fn badge_mark(
    title: impl Into<String>,
    variant: Variant,
    tea: Tokens,
    name: &str,
) -> Element<'static, Message> {
    let title = title.into();
    widget::badge(
        title,
        None,
        tea,
        variant,
        widget::BadgeSize::Small,
        A11y::new(name, Role::Status),
    )
}

fn detail_panel(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let main = column![issue_header(palette, tea), issue_main(palette, tea)]
        .spacing(8)
        .width(Fill)
        .height(Fill);
    let body = row![
        container(main).width(Length::FillPortion(3)).height(Fill),
        container(Space::new().width(1).height(Fill)).style(move |_| icedtea::style::hairline(tea)),
        container(side_pane(palette, tea))
            .width(Length::FillPortion(2))
            .height(Fill),
    ]
    .spacing(12)
    .width(Fill)
    .height(Fill);
    container(body.padding(12))
        .width(Fill)
        .height(Fill)
        .style(move |_| icedtea::style::panel(tea))
        .into()
}

fn issue_header(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let Some(issue) = palette.header_issue() else {
        return Space::new().width(Fill).into();
    };
    let title_color = if issue.state == "DONE" {
        tea.muted
    } else {
        tea.text
    };
    column![
        row![
            issue_marks(
                Some(issue.priority),
                issue.state,
                issue.blocked,
                issue.claimed,
                tea,
            ),
            project_mark(issue.project, tea),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(issue.title.to_string())
            .size(tea.body())
            .font(icedtea::typo::UI)
            .color(title_color)
            .wrapping(iced::widget::text::Wrapping::Word)
            .width(Fill),
    ]
    .spacing(6)
    .width(Fill)
    .into()
}

fn side_pane<'a>(palette: &'a Palette, tea: Tokens) -> Element<'a, Message> {
    let titles: Vec<String> = DetailTab::ALL
        .iter()
        .map(|t| t.label().to_string())
        .collect();
    let active = DetailTab::ALL
        .iter()
        .position(|t| *t == palette.detail_tab())
        .unwrap_or(0);
    iced::widget::responsive(move |size| {
        let mut tabs = Tabs::new(titles.clone());
        tabs.select(active);
        tabs.closable = false;
        let bar = widget::tab_bar(
            &tabs,
            |i| Message::DetailTab(DetailTab::ALL[i.min(DetailTab::ALL.len() - 1)]),
            |_| Message::Noop,
            size.width,
            false,
            tea,
            A11y::new("detail", Role::Tab),
        );
        let content = pane_scroll(
            detail_tab_body(palette, tea),
            tea,
            A11y::new("detail-scroll", Role::Group),
        );
        column![bar, content].spacing(8).height(Fill).into()
    })
    .width(Fill)
    .height(Fill)
    .into()
}

fn list_detail<'a>(
    palette: &'a Palette,
    list: Element<'a, Message>,
    tea: Tokens,
) -> Element<'a, Message> {
    icedtea::layout::split_view(
        list,
        detail_panel(palette, tea),
        palette.detail_split(),
        palette.split_total(),
        Message::Sash,
        tea.direction,
        tea,
    )
}

fn note_bar(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let draft = palette.note_draft().unwrap_or("");
    widget::themed_text_input(
        "Add a note to the logbook",
        draft,
        Message::NoteChanged,
        Some(Message::NoteSubmit),
        widget::FieldOpts::NONE,
        tea,
        A11y::new("note", Role::TextBox),
        None,
    )
}

fn help_overlay(palette: &Palette, tea: Tokens) -> Element<'_, Message> {
    let doc = palette.help_md();
    container(
        column![
            widget::label("vissue", tea, A11y::new("help", Role::Header)),
            widget::themed_scroll(
                widget::markdown_view(
                    &doc.items,
                    None,
                    |_| Message::Noop,
                    tea,
                    |url| Message::MdLink(url.to_string()),
                    A11y::new("help-md", Role::Group),
                ),
                tea,
                A11y::new("help-scroll", Role::Group),
                false,
                None,
                None::<fn(f32) -> Message>,
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

fn detail_tab_body<'a>(palette: &'a Palette, tea: Tokens) -> Element<'a, Message> {
    match palette.detail_tab() {
        DetailTab::Tree => tree_list(palette, tea),
        DetailTab::Related => related_list(palette, tea),
        DetailTab::Notes => notes_body(palette, tea),
    }
}

fn select_field<'a>(
    palette: &'a Palette,
    id: &str,
    face: FontFace,
    tea: Tokens,
) -> Option<Element<'a, Message>> {
    let content = palette.selectable(id)?;
    let field = id.to_string();
    Some(widget::selectable(
        content,
        {
            let field = field.clone();
            move |action| Message::SelectField(field.clone(), action)
        },
        tea,
        face,
        A11y::new(field, Role::TextBox),
    ))
}

fn tree_list<'a>(palette: &'a Palette, tea: Tokens) -> Element<'a, Message> {
    let rows = palette.tree_rows();
    if rows.is_empty() {
        return widget::meta(
            tab_empty_copy(DetailTab::Tree),
            tea,
            A11y::new("detail-body", Role::Group),
        );
    }
    let selected = palette.tree_selected();
    let mut col = column![].spacing(2);
    for row in rows {
        col = col.push(tree_row(palette, row, selected == Some(row.tea_id), tea));
    }
    col.into()
}

fn notes_body<'a>(palette: &'a Palette, tea: Tokens) -> Element<'a, Message> {
    let Some(detail) = palette.detail() else {
        return widget::meta(
            tab_empty_copy(DetailTab::Notes),
            tea,
            A11y::new("notes", Role::Group),
        );
    };
    if detail.logbook.is_empty() {
        return widget::meta(
            tab_empty_copy(DetailTab::Notes),
            tea,
            A11y::new("notes", Role::Group),
        );
    }
    let mut col = column![].spacing(6);
    for (i, entry) in detail.logbook.iter().enumerate() {
        let mut card = column![].spacing(4).width(Fill);
        if !entry.timestamp.is_empty() {
            card = card.push(widget::meta(
                crate::dates::format_org_stamps(&entry.timestamp),
                tea,
                A11y::new("when", Role::Status),
            ));
        }
        if let (Some(to), from) = (&entry.to_state, &entry.from_state) {
            let mut flip = row![].spacing(6).align_y(Alignment::Center);
            if let Some(from) = from {
                flip = flip.push(badge_state(from, tea));
            }
            flip = flip.push(
                text(state_arrow(tea.direction))
                    .size(tea.meta())
                    .color(tea.muted),
            );
            flip = flip.push(badge_state(to, tea));
            card = card.push(flip);
        }
        if let Some(note) = select_field(palette, &format!("note-{i}"), FontFace::Ui, tea) {
            card = card.push(note);
        }
        if let Some(clock) = select_field(palette, &format!("clock-{i}"), FontFace::Mono, tea) {
            card = card.push(clock);
        }
        col = col.push(
            container(card.padding([6, 4]))
                .width(Fill)
                .style(move |_| icedtea::style::card(tea, false)),
        );
    }
    col.into()
}

fn issue_main<'a>(palette: &'a Palette, tea: Tokens) -> Element<'a, Message> {
    pane_scroll(
        column![issue_fields(palette, tea), issue_prose(palette, tea)]
            .spacing(12)
            .width(Fill)
            .into(),
        tea,
        A11y::new("issue", Role::Group),
    )
}

fn issue_fields<'a>(palette: &'a Palette, tea: Tokens) -> Element<'a, Message> {
    if palette.excerpt().is_none_or(|excerpt| excerpt.suppressed) || palette.detail().is_none() {
        return column![].into();
    }
    let label_w = palette.excerpt_label_width();
    let mut table = column![].spacing(6).width(Fill);
    for (id, label) in palette.excerpt_form() {
        let Some(content) = palette.selectable(&id) else {
            continue;
        };
        let bind = id.clone();
        table = table.push(widget::value_field(
            label,
            content,
            move |action| Message::SelectField(bind.clone(), action),
            None,
            FontFace::Ui,
            label_w,
            tea,
            Direction::Ltr,
            A11y::new(id, Role::Group),
        ));
    }
    table.into()
}

fn issue_prose<'a>(palette: &'a Palette, tea: Tokens) -> Element<'a, Message> {
    let Some(excerpt) = palette.excerpt() else {
        return widget::meta(
            if palette.detail_body().is_empty() {
                "Select a row."
            } else {
                palette.detail_body()
            },
            tea,
            A11y::new("excerpt", Role::Group),
        );
    };
    if excerpt.suppressed {
        return widget::meta(
            excerpt.text.as_str(),
            tea,
            A11y::new("excerpt", Role::Group),
        );
    }
    let Some(detail) = palette.detail() else {
        return widget::meta(
            if palette.detail_body().is_empty() {
                "Select a row."
            } else {
                palette.detail_body()
            },
            tea,
            A11y::new("excerpt", Role::Group),
        );
    };
    if detail.body.trim().is_empty() {
        return widget::meta("No body.", tea, A11y::new("body", Role::Status));
    }
    select_field(palette, "excerpt-body", FontFace::Ui, tea)
        .unwrap_or_else(|| widget::meta("No body.", tea, A11y::new("body", Role::Status)))
}

fn related_list<'a>(palette: &'a Palette, tea: Tokens) -> Element<'a, Message> {
    let hits = palette.related_hits();
    if hits.is_empty() {
        return widget::meta(
            tab_empty_copy(DetailTab::Related),
            tea,
            A11y::new("related", Role::Group),
        );
    }
    let mut col = column![].spacing(4);
    let selected = palette.selected_id();
    for hit in hits {
        let (priority, blocked, claimed) = palette
            .related_marks(&hit.id)
            .map(|(p, b, c)| (Some(p), b, c))
            .unwrap_or((None, false, false));
        let why = related_why(&hit.evidence);
        let id = hit.id.clone();
        let title_color = if hit.state == "DONE" {
            tea.muted
        } else {
            tea.text
        };
        let marks = row![
            issue_marks(priority, &hit.state, blocked, claimed, tea),
            project_mark(&hit.project, tea),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Fill);
        let mut body = column![
            marks,
            text(hit.title.clone())
                .size(tea.body())
                .font(icedtea::typo::UI)
                .color(title_color)
                .wrapping(iced::widget::text::Wrapping::Word)
                .width(Fill),
        ]
        .spacing(4)
        .width(Fill)
        .align_x(Alignment::Start);
        if !why.is_empty() {
            body = body.push(widget::meta(why, tea, A11y::new("why", Role::Status)));
        }
        let selected = selected == Some(hit.id.as_str());
        col = col.push(
            mouse_area(
                container(body.padding([8, 6]))
                    .width(Fill)
                    .style(move |_| icedtea::style::card(tea, selected)),
            )
            .on_press(Message::OpenIssue(id)),
        );
    }
    col.into()
}

fn related_why(evidence: &[String]) -> String {
    evidence
        .iter()
        .filter_map(|reason| {
            if reason.starts_with("score") {
                return None;
            }
            if reason == "blocked_by" || reason.starts_with("blocked") {
                return Some("blocked".to_string());
            }
            if reason.starts_with("term:") {
                return None;
            }
            if let Some(n) = reason.strip_prefix("org_distance:") {
                return Some(if n == "1" {
                    "nearby".to_string()
                } else {
                    format!("distance {n}")
                });
            }
            Some(reason.clone())
        })
        .collect::<Vec<_>>()
        .join("  ·  ")
}

fn state_arrow(dir: Direction) -> &'static str {
    match dir {
        Direction::Ltr => "→",
        Direction::Rtl => "←",
    }
}

fn tab_empty_copy(tab: DetailTab) -> &'static str {
    match tab {
        DetailTab::Tree => "No parent or child links.",
        DetailTab::Related => "No related issues.",
        DetailTab::Notes => "No logbook yet. n writes a note.",
    }
}

fn pane_scroll<'a>(child: Element<'a, Message>, tea: Tokens, a11y: A11y) -> Element<'a, Message> {
    // themed_scroll always paints a rail, even when the pane fits.
    // iced overlay scrollbars appear only on overflow.
    icedtea::a11y::attach(
        iced::widget::scrollable(child)
            .style(icedtea::style::scroll_style(tea))
            .width(Fill)
            .height(Fill)
            .into(),
        &a11y,
    )
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
    use crate::palette::DetailTab;

    #[test]
    fn board_paints_through_icedtea() {
        let src = include_str!("view.rs");
        assert!(
            src.contains("icedtea::widget::markdown_view") || src.contains("widget::markdown_view")
        );
        assert!(src.contains("widget::chip"));
        assert!(src.contains("widget::themed_text_input"));
        assert!(src.contains("widget::search_input_clear"));
        assert!(src.contains("widget::list_view"));
        assert!(src.contains("widget::expander"));
        assert!(src.contains("widget::themed_checkbox"));
        assert!(src.contains("split_view"));
        assert!(src.contains("fn tree_row"));
        assert!(src.contains("fn issue_marks"));
        assert!(src.contains("widget::badge"));
        assert!(src.contains("Variant::Success"));
        assert!(src.contains("fn actions"));
        assert!(src.contains("fn related_list"));
        assert!(src.contains("fn issue_header"));
        assert!(src.contains("fn issue_main"));
        assert!(src.contains("fn issue_fields"));
        assert!(src.contains("fn issue_prose"));
        assert!(src.contains("FillPortion(3)"));
        assert!(src.contains("FillPortion(2)"));
        let prod = src.split("#[cfg(test)]").next().unwrap();
        assert!(!prod.contains("TAB_FIT"));
        assert!(!prod.contains("920"));
        assert!(src.contains("fn side_pane"));
        assert!(src.contains("widget::tab_bar"));
        assert!(src.contains("responsive"));
        assert!(src.contains("size.width"));
        assert!(src.contains("Wrapping::Word"));
        assert!(src.contains("widget::selectable"));
        assert!(src.contains("widget::value_field"));
        assert!(src.contains("excerpt_label_width"));
        assert!(src.contains("fn notes_body"));
        assert!(src.contains("fn state_arrow"));
        assert!(src.contains("\"→\""));
        assert!(src.contains("\"←\""));
        assert!(src.contains("icedtea::typo::MONO"));
        assert!(src.contains("on_double_click"));
        assert!(src.contains("Message::OpenIssue"));
        assert!(src.contains("icedtea::style::card"));
        assert!(src.contains("Variant::Chip"));
        let prod = src.split("fn project_mark").nth(1).unwrap();
        let prod = prod.split("fn badge_priority").next().unwrap();
        assert!(prod.contains("Variant::Chip"));
        assert!(!prod.contains("Variant::Danger"));
    }

    #[test]
    fn side_tab_titles_all_fit_a_360_pane() {
        let titles: Vec<String> = DetailTab::ALL
            .iter()
            .map(|t| t.label().to_string())
            .collect();
        assert_eq!(icedtea::widget::tab_visible_count(&titles, 360.0), 3);
        assert_eq!(icedtea::widget::tab_visible_count(&titles, 960.0), 3);
    }
}
