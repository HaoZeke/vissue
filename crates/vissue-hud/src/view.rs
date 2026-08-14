//! iced widgets for the task board. Drawing only; logic lives in [`crate::palette`].

use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length};

use crate::app::Message;
use crate::palette::{BoardFilter, DetailTab, Focus, HudItem, Palette, ProjectSection};
use crate::theme;

/// Board face. Hidden state draws nothing so a `Mode::Hidden` frame is empty.
pub fn view(palette: &Palette) -> Element<'_, Message> {
    if !palette.visible() {
        return Space::new().width(0).height(0).into();
    }
    if palette.focus() == Focus::Help {
        return help_overlay(palette);
    }

    let mut pane = column![header(palette), chips(palette)]
        .spacing(10)
        .padding([14, 16]);

    if palette.browsing() {
        pane = pane.push(project_browser(palette));
    } else {
        pane = pane.push(add_bar(palette));
        let sections = palette.sections();
        if sections.is_empty() {
            pane = pane.push(
                text(empty_copy(palette))
                    .size(theme::SIZE_BODY)
                    .color(theme::SUBTEXT)
                    .font(theme::FACE),
            );
        } else {
            let mut list = column![].spacing(4);
            let selected = palette.selected_index();
            for section in sections {
                if palette.project().is_none() {
                    list = list.push(project_header(&section));
                }
                if section.collapsed {
                    continue;
                }
                for (i, item) in section.rows {
                    list = list.push(task_row(item, i == selected));
                }
            }
            pane = pane.push(scrollable(list).height(Length::FillPortion(5)));
        }
        if palette.selected_id().is_some() {
            pane = pane.push(detail_panel(palette));
        }
    }

    if palette.note_draft().is_some() {
        pane = pane.push(note_bar(palette));
    }
    if let Some(kind) = palette.confirm() {
        pane = pane.push(
            text(format!("confirm {}? y/n", kind.state()))
                .size(theme::SIZE_BODY)
                .color(theme::PEACH)
                .font(theme::FACE),
        );
    }
    if !palette.message().is_empty() {
        pane = pane.push(
            text(palette.message())
                .size(theme::SIZE_META)
                .color(theme::PEACH)
                .font(theme::FACE),
        );
    }

    container(pane.spacing(10).width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_| fill(theme::BASE))
        .into()
}

fn header(palette: &Palette) -> Element<'_, Message> {
    let live = match palette.serve_status() {
        vissue_tui::attach::ServeStatus::Live => theme::GREEN,
        vissue_tui::attach::ServeStatus::Mismatch => theme::PEACH,
        vissue_tui::attach::ServeStatus::Offline => theme::OVERLAY,
    };
    let dot = container(Space::new().width(8).height(8)).style(move |_| container::Style {
        background: Some(Background::Color(live)),
        border: Border {
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..container::Style::default()
    });
    let scope = if palette.browsing() {
        "projects"
    } else {
        palette.project().unwrap_or("search")
    };
    let mut title = row![
        text("vissue")
            .size(theme::SIZE_TITLE)
            .color(theme::TEXT)
            .font(theme::FACE),
        dot,
        text(scope)
            .size(theme::SIZE_META)
            .color(theme::SUBTEXT)
            .font(theme::FACE),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if palette.project().is_some() {
        title = title.push(
            button(
                text("Projects")
                    .size(theme::SIZE_META)
                    .color(theme::SUBTEXT)
                    .font(theme::FACE),
            )
            .on_press(Message::LeaveProject)
            .padding([4, 10])
            .style(|_, status| chip_style(status, false)),
        );
    }

    row![title, Space::new().width(Fill), find_control(palette)]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

fn find_control(palette: &Palette) -> Element<'_, Message> {
    if palette.focus() == Focus::Search || palette.filter() == BoardFilter::Search {
        text_input("Search title, id, tags", palette.query())
            .on_input(Message::QueryChanged)
            .on_submit(Message::FocusList)
            .size(theme::SIZE_META)
            .padding([8, 10])
            .font(theme::FACE)
            .width(260)
            .style(|_, _| input_style())
            .into()
    } else {
        button(
            text("Search")
                .size(theme::SIZE_META)
                .color(theme::SUBTEXT)
                .font(theme::FACE),
        )
        .on_press(Message::FocusSearch)
        .padding([8, 12])
        .style(|_, status| chip_style(status, false))
        .into()
    }
}

fn chips(palette: &Palette) -> Element<'_, Message> {
    let mut row = row![].spacing(6);
    if palette.browsing() {
        row = row.push(projects_chip(palette.project_cards().len(), true));
        for (filter, label) in [
            (BoardFilter::Claims, "Claims"),
            (BoardFilter::Agenda, "Agenda"),
            (BoardFilter::Search, "Search"),
        ] {
            row = row.push(chip(filter, label, palette.count(filter), false));
        }
        return row.into();
    }
    row = row.push(projects_chip(0, false));
    for (filter, label) in BoardFilter::ALL {
        row = row.push(chip(
            filter,
            label,
            palette.count(filter),
            palette.filter() == filter,
        ));
    }
    row.into()
}

fn projects_chip(count: usize, active: bool) -> Element<'static, Message> {
    let fg = if active { theme::BASE } else { theme::SUBTEXT };
    let label = if active && count > 0 {
        format!("Projects {count}")
    } else {
        "Projects".to_string()
    };
    button(
        text(label)
            .size(theme::SIZE_META)
            .color(fg)
            .font(theme::FACE),
    )
    .on_press(Message::LeaveProject)
    .padding([6, 12])
    .style(move |_, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(Background::Color(if active {
                theme::BLUE
            } else if hovered {
                theme::SURFACE1
            } else {
                theme::SURFACE0
            })),
            text_color: fg,
            border: Border {
                radius: 14.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..button::Style::default()
        }
    })
    .into()
}

fn project_browser(palette: &Palette) -> Element<'_, Message> {
    let cards = palette.project_cards();
    if cards.is_empty() {
        return text("No projects under this prefix.")
            .size(theme::SIZE_BODY)
            .color(theme::SUBTEXT)
            .font(theme::FACE)
            .into();
    }
    let selected = palette.selected_project_index();
    let mut list = column![].spacing(8);
    for (i, card) in cards.iter().enumerate() {
        list = list.push(project_card(card, i == selected));
    }
    scrollable(list).height(Length::Fill).into()
}

fn project_card(card: &crate::palette::ProjectCard, selected: bool) -> Element<'static, Message> {
    let name = card.name.clone();
    let count = match card.ready {
        0 => "caught up".to_string(),
        1 => "1 ready".to_string(),
        n => format!("{n} ready"),
    };
    let title_fg = if selected { theme::BASE } else { theme::TEXT };
    let meta_fg = if selected {
        theme::MANTLE
    } else if card.ready == 0 {
        theme::OVERLAY
    } else {
        theme::SUBTEXT
    };
    let body = row![
        column![
            text(card.name.clone())
                .size(theme::SIZE_TITLE)
                .color(title_fg)
                .font(theme::FACE),
            text(count)
                .size(theme::SIZE_META)
                .color(meta_fg)
                .font(theme::FACE),
        ]
        .spacing(2),
        Space::new().width(Fill),
        text("open")
            .size(theme::SIZE_META)
            .color(meta_fg)
            .font(theme::FACE),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([12, 14]);
    button(body)
        .on_press(Message::SelectProject(name))
        .padding(0)
        .width(Fill)
        .style(move |_, status| select_style(selected, matches!(status, button::Status::Hovered)))
        .into()
}

fn chip(
    filter: BoardFilter,
    label: &'static str,
    count: usize,
    active: bool,
) -> Element<'static, Message> {
    let fg = if active { theme::BASE } else { theme::SUBTEXT };
    button(
        text(if count > 0 {
            format!("{label} {count}")
        } else {
            label.to_string()
        })
        .size(theme::SIZE_META)
        .color(fg)
        .font(theme::FACE),
    )
    .on_press(Message::Filter(filter))
    .padding([6, 12])
    .style(move |_, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(Background::Color(if active {
                theme::BLUE
            } else if hovered {
                theme::SURFACE1
            } else {
                theme::SURFACE0
            })),
            text_color: fg,
            border: Border {
                radius: 14.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..button::Style::default()
        }
    })
    .into()
}

fn add_bar(palette: &Palette) -> Element<'_, Message> {
    if palette.focus() == Focus::Add {
        text_input("Add a task in the current project", palette.add_draft())
            .on_input(Message::AddChanged)
            .on_submit(Message::AddSubmit)
            .size(theme::SIZE_BODY)
            .padding(12)
            .font(theme::FACE)
            .style(|_, _| input_style())
            .into()
    } else {
        button(
            row![
                text("+")
                    .size(theme::SIZE_TITLE)
                    .color(theme::BLUE)
                    .font(theme::FACE),
                text("Add a task")
                    .size(theme::SIZE_BODY)
                    .color(theme::OVERLAY)
                    .font(theme::FACE),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .on_press(Message::FocusAdd)
        .padding([10, 12])
        .width(Fill)
        .style(|_, status| {
            let hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(Background::Color(if hovered {
                    theme::SURFACE1
                } else {
                    theme::SURFACE0
                })),
                text_color: theme::OVERLAY,
                border: Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: if hovered {
                        theme::BLUE
                    } else {
                        theme::SURFACE1
                    },
                },
                ..button::Style::default()
            }
        })
        .into()
    }
}

fn project_header(section: &ProjectSection<'_>) -> Element<'static, Message> {
    let mark = if section.collapsed { "+" } else { "-" };
    let name = section.project.to_string();
    let label = format!("{mark}  {}  {}", section.project, section.rows.len());
    button(
        text(label)
            .size(theme::SIZE_BODY)
            .color(theme::SUBTEXT)
            .font(theme::FACE),
    )
    .on_press(Message::ToggleProject(name))
    .padding([8, 10])
    .width(Fill)
    .style(|_, status| {
        let hovered = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(Background::Color(if hovered {
                theme::SURFACE0
            } else {
                theme::MANTLE
            })),
            text_color: theme::SUBTEXT,
            border: Border {
                radius: 8.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..button::Style::default()
        }
    })
    .into()
}

fn task_row(item: &HudItem, selected: bool) -> Element<'_, Message> {
    let done = item.state == "DONE";
    let title_color = if selected {
        theme::BASE
    } else if done {
        theme::OVERLAY
    } else {
        theme::TEXT
    };
    let meta_color = if selected {
        theme::MANTLE
    } else {
        theme::state_color(&item.state)
    };
    let id = item.id.clone();
    let pip_color = theme::priority_color(&item.priority);
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

    let pip = container(Space::new().width(4).height(34)).style(move |_| container::Style {
        background: Some(Background::Color(pip_color)),
        border: Border {
            radius: 2.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..container::Style::default()
    });

    let titles = column![
        text(item.title.clone())
            .size(theme::SIZE_BODY)
            .color(title_color)
            .font(theme::FACE),
        text(meta)
            .size(theme::SIZE_META)
            .color(meta_color)
            .font(theme::FACE),
    ]
    .spacing(2);

    let indent = (item.depth as f32) * 18.0;
    let body = row![
        Space::new().width(indent),
        checkbox(done)
            .on_toggle(move |_| Message::ToggleDone(id.clone()))
            .size(18)
            .style(move |_, status| check_style(status, done)),
        pip,
        titles,
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .padding([8, 6]);

    let idx = item.id.clone();
    button(body)
        .on_press(Message::SelectId(idx))
        .padding(0)
        .width(Fill)
        .style(move |_, status| select_style(selected, matches!(status, button::Status::Hovered)))
        .into()
}

fn detail_panel(palette: &Palette) -> Element<'_, Message> {
    let mut tabs = row![].spacing(6);
    for tab in DetailTab::ALL {
        let active = palette.detail_tab() == tab;
        tabs = tabs.push(
            button(
                text(tab.label())
                    .size(theme::SIZE_META)
                    .color(if active { theme::BASE } else { theme::SUBTEXT })
                    .font(theme::FACE),
            )
            .on_press(Message::DetailTab(tab))
            .padding([4, 10])
            .style(move |_, status| chip_style(status, active)),
        );
    }

    let body = if palette.detail_body().is_empty() {
        "Select a row."
    } else {
        palette.detail_body()
    };

    container(
        column![
            tabs,
            scrollable(
                text(body)
                    .size(theme::SIZE_META)
                    .color(theme::TEXT)
                    .font(theme::FACE)
            )
            .height(Length::Fill),
        ]
        .spacing(8)
        .padding(12),
    )
    .width(Fill)
    .height(Length::Fixed(220.0))
    .style(|_| container::Style {
        background: Some(Background::Color(theme::MANTLE)),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: theme::OVERLAY1,
        },
        ..container::Style::default()
    })
    .into()
}

fn note_bar(palette: &Palette) -> Element<'_, Message> {
    let draft = palette.note_draft().unwrap_or("");
    text_input("Add a note to the logbook", draft)
        .on_input(Message::NoteChanged)
        .on_submit(Message::NoteSubmit)
        .size(theme::SIZE_BODY)
        .padding(10)
        .font(theme::FACE)
        .style(|_, _| input_style())
        .into()
}

fn help_overlay(palette: &Palette) -> Element<'_, Message> {
    container(
        column![
            text("vissue")
                .size(theme::SIZE_TITLE)
                .color(theme::TEXT)
                .font(theme::FACE),
            scrollable(
                text(palette.help_text())
                    .size(theme::SIZE_META)
                    .color(theme::SUBTEXT)
                    .font(theme::FACE)
            ),
            text("esc closes help")
                .size(theme::SIZE_HINT)
                .color(theme::OVERLAY)
                .font(theme::FACE),
        ]
        .spacing(12)
        .padding(24),
    )
    .width(Fill)
    .height(Fill)
    .style(|_| fill(theme::BASE))
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

/// Rofi selected-normal: blue field, mocha text.
fn select_style(selected: bool, hovered: bool) -> button::Style {
    let (bg, fg) = if selected {
        (theme::BLUE, theme::BASE)
    } else if hovered {
        (theme::SURFACE0, theme::TEXT)
    } else {
        (Color::TRANSPARENT, theme::TEXT)
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: Border {
            radius: 8.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..button::Style::default()
    }
}

fn fill(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color)),
        text_color: Some(theme::TEXT),
        ..container::Style::default()
    }
}

fn input_style() -> text_input::Style {
    text_input::Style {
        background: Background::Color(theme::SURFACE0),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: theme::OVERLAY1,
        },
        icon: theme::SUBTEXT,
        placeholder: theme::OVERLAY,
        value: theme::TEXT,
        selection: theme::BLUE,
    }
}

fn chip_style(status: button::Status, active: bool) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered);
    button::Style {
        background: Some(Background::Color(if active {
            theme::BLUE
        } else if hovered {
            theme::SURFACE1
        } else {
            theme::SURFACE0
        })),
        text_color: if active { theme::BASE } else { theme::SUBTEXT },
        border: Border {
            radius: 12.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..button::Style::default()
    }
}

fn check_style(status: checkbox::Status, done: bool) -> checkbox::Style {
    let hovered = matches!(status, checkbox::Status::Hovered { .. });
    checkbox::Style {
        background: Background::Color(if done {
            theme::GREEN
        } else if hovered {
            theme::SURFACE1
        } else {
            Color::TRANSPARENT
        }),
        icon_color: theme::BASE,
        border: Border {
            radius: 6.0.into(),
            width: 2.0,
            color: if done { theme::GREEN } else { theme::SUBTEXT },
        },
        text_color: Some(theme::TEXT),
    }
}
