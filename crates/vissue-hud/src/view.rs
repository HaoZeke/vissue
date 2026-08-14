//! iced widgets for the task board. Drawing only; logic lives in [`crate::palette`].

use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length};

use crate::app::Message;
use crate::palette::{BoardFilter, Focus, HudItem, Palette};
use crate::theme;

/// Board face. Hidden state draws nothing so a `Mode::Hidden` frame is empty.
pub fn view(palette: &Palette) -> Element<'_, Message> {
    if !palette.visible() {
        return Space::new().width(0).height(0).into();
    }

    let body = row![sidebar(palette), main_pane(palette)]
        .width(Fill)
        .height(Fill);

    container(body)
        .width(Fill)
        .height(Fill)
        .style(|_| fill(theme::BASE))
        .into()
}

fn sidebar(palette: &Palette) -> Element<'_, Message> {
    let mut nav = column![
        text("vissue")
            .size(theme::SIZE_TITLE)
            .color(theme::TEXT)
            .font(theme::FACE),
        text(status_chip(palette))
            .size(theme::SIZE_HINT)
            .color(theme::OVERLAY)
            .font(theme::FACE),
    ]
    .spacing(10);

    for (filter, label) in BoardFilter::ALL {
        let count = palette.count(filter);
        let active = palette.filter() == filter;
        nav = nav.push(filter_btn(filter, label, count, active));
    }

    container(nav.spacing(8).padding(18).width(Fill).height(Fill))
        .width(220)
        .height(Fill)
        .style(|_| fill(theme::MANTLE))
        .into()
}

fn filter_btn(
    filter: BoardFilter,
    label: &'static str,
    count: usize,
    active: bool,
) -> Element<'static, Message> {
    let fg = if active { theme::TEXT } else { theme::SUBTEXT };
    let label_row = row![
        text(label)
            .size(theme::SIZE_BODY)
            .color(fg)
            .font(theme::FACE),
        Space::new().width(Fill),
        text(format!("{count}"))
            .size(theme::SIZE_META)
            .color(if active { theme::BLUE } else { theme::OVERLAY })
            .font(theme::FACE),
    ]
    .align_y(Alignment::Center);

    button(label_row)
        .on_press(Message::Filter(filter))
        .padding([10, 12])
        .width(Fill)
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(Background::Color(if active {
                    theme::SURFACE0
                } else if hovered {
                    theme::SURFACE1
                } else {
                    Color::TRANSPARENT
                })),
                text_color: fg,
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

fn main_pane(palette: &Palette) -> Element<'_, Message> {
    let heading = format!(
        "{}  {}",
        palette.filter().label(),
        palette.filtered_items().len()
    );
    let mut pane = column![
        text(heading)
            .size(theme::SIZE_TITLE)
            .color(theme::TEXT)
            .font(theme::FACE),
        add_bar(palette),
        search_bar(palette),
    ]
    .spacing(10)
    .padding(18);

    let items = palette.filtered_items();
    if items.is_empty() {
        pane = pane.push(
            text(empty_copy(palette))
                .size(theme::SIZE_BODY)
                .color(theme::SUBTEXT)
                .font(theme::FACE),
        );
    } else {
        let mut list = column![].spacing(4);
        for (i, item) in items.into_iter().enumerate() {
            list = list.push(task_row(item, i == palette.selected_index()));
        }
        pane = pane.push(scrollable(list).height(Length::Fill));
    }

    if let Some(excerpt) = palette.excerpt() {
        pane = pane.push(excerpt_card(palette, excerpt.text.as_str()));
    }

    if palette.note_draft().is_some() {
        pane = pane.push(note_bar(palette));
    }

    if !palette.message().is_empty() {
        pane = pane.push(
            text(palette.message())
                .size(theme::SIZE_META)
                .color(theme::PEACH)
                .font(theme::FACE),
        );
    }

    pane = pane.push(
        text("space done   enter open   c claim   n note   a add   / find   1-4 lists   esc")
            .size(theme::SIZE_HINT)
            .color(theme::OVERLAY)
            .font(theme::FACE),
    );

    container(pane.spacing(10).width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_| fill(theme::BASE))
        .into()
}

fn add_bar(palette: &Palette) -> Element<'_, Message> {
    if palette.focus() == Focus::Add {
        text_input("Add a task", palette.add_draft())
            .on_input(Message::AddChanged)
            .on_submit(Message::AddSubmit)
            .size(theme::SIZE_BODY)
            .padding(12)
            .font(theme::FACE)
            .style(|_, _| input_style())
            .into()
    } else {
        button(
            text("Add a task")
                .size(theme::SIZE_BODY)
                .color(theme::OVERLAY)
                .font(theme::FACE),
        )
        .on_press(Message::FocusAdd)
        .padding(12)
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
                    radius: 10.0.into(),
                    width: 1.0,
                    color: theme::SURFACE1,
                },
                ..button::Style::default()
            }
        })
        .into()
    }
}

fn search_bar(palette: &Palette) -> Element<'_, Message> {
    if palette.focus() == Focus::Search || !palette.query().is_empty() {
        text_input("Find", palette.query())
            .on_input(Message::QueryChanged)
            .on_submit(Message::FocusList)
            .size(theme::SIZE_BODY)
            .padding(10)
            .font(theme::FACE)
            .style(|_, _| input_style())
            .into()
    } else {
        button(
            text("Find")
                .size(theme::SIZE_META)
                .color(theme::OVERLAY)
                .font(theme::FACE),
        )
        .on_press(Message::FocusSearch)
        .padding([8, 12])
        .width(Fill)
        .style(|_, _| button::Style {
            background: Some(Background::Color(theme::MANTLE)),
            text_color: theme::OVERLAY,
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: theme::SURFACE1,
            },
            ..button::Style::default()
        })
        .into()
    }
}

fn task_row(item: &HudItem, selected: bool) -> Element<'_, Message> {
    let done = item.state == "DONE";
    let title_color = if done { theme::OVERLAY } else { theme::TEXT };
    let id = item.id.clone();
    let pip_color = theme::priority_color(&item.priority);
    let mut meta = format!("{}  {}  {}", item.project, item.state, item.id);
    if let Some(due) = item.due.as_deref() {
        meta = format!("{meta}  {due}");
    }
    if let Some(holder) = item.claimed_by.as_deref() {
        meta = format!("{meta}  @{holder}");
    }

    let pip = container(Space::new().width(4).height(28))
        .style(move |_| container::Style {
            background: Some(Background::Color(pip_color)),
            border: Border {
                radius: 2.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..container::Style::default()
        })
        .width(4)
        .height(28);

    let titles = column![
        text(item.title.clone())
            .size(theme::SIZE_TITLE)
            .color(title_color)
            .font(theme::FACE),
        text(meta)
            .size(theme::SIZE_META)
            .color(theme::state_color(&item.state))
            .font(theme::FACE),
    ]
    .spacing(2);

    let body = row![
        checkbox(done)
            .on_toggle(move |_| Message::ToggleDone(id.clone()))
            .size(18)
            .style(move |_, status| check_style(status, done)),
        pip,
        titles,
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([8, 10]);

    let idx = item.id.clone();
    button(body)
        .on_press(Message::SelectId(idx))
        .padding(0)
        .width(Fill)
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(Background::Color(if selected {
                    theme::SURFACE0
                } else if hovered {
                    theme::MANTLE
                } else {
                    Color::TRANSPARENT
                })),
                text_color: title_color,
                border: Border {
                    radius: 10.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..button::Style::default()
            }
        })
        .into()
}

fn excerpt_card<'a>(palette: &'a Palette, body: &'a str) -> Element<'a, Message> {
    let label = palette
        .detail()
        .map(|d| d.title.clone())
        .unwrap_or_else(|| "note".into());
    container(
        column![
            text(label)
                .size(theme::SIZE_BODY)
                .color(theme::BLUE)
                .font(theme::FACE),
            scrollable(
                text(body)
                    .size(theme::SIZE_META)
                    .color(theme::TEXT)
                    .font(theme::FACE)
            )
            .height(Length::Fixed(160.0)),
        ]
        .spacing(8)
        .padding(14),
    )
    .width(Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(theme::MANTLE)),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: theme::SURFACE1,
        },
        ..container::Style::default()
    })
    .into()
}

fn note_bar(palette: &Palette) -> Element<'_, Message> {
    let draft = palette.note_draft().unwrap_or("");
    text_input("Add a note", draft)
        .on_input(Message::NoteChanged)
        .on_submit(Message::NoteSubmit)
        .size(theme::SIZE_BODY)
        .padding(10)
        .font(theme::FACE)
        .style(|_, _| input_style())
        .into()
}

fn empty_copy(palette: &Palette) -> &'static str {
    if !palette.query().is_empty() {
        return "Nothing matches that find.";
    }
    match palette.filter() {
        BoardFilter::Ready => "Nothing ready. Add a task or open another list.",
        BoardFilter::Mine => "Nothing claimed by you.",
        BoardFilter::Upcoming => "Nothing dated in the next two weeks.",
        BoardFilter::All => "The vault is empty.",
    }
}

fn status_chip(palette: &Palette) -> String {
    let kind = match palette.serve_status() {
        vissue_tui::attach::ServeStatus::Live => "live",
        vissue_tui::attach::ServeStatus::Offline => "offline",
        vissue_tui::attach::ServeStatus::Mismatch => "mismatch",
    };
    format!("{kind}  {}", palette.agent())
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
            radius: 10.0.into(),
            width: 1.0,
            color: theme::SURFACE1,
        },
        icon: theme::SUBTEXT,
        placeholder: theme::OVERLAY,
        value: theme::TEXT,
        selection: theme::BLUE,
    }
}

fn check_style(status: checkbox::Status, done: bool) -> checkbox::Style {
    let accent = if done { theme::GREEN } else { theme::SURFACE1 };
    let hovered = matches!(status, checkbox::Status::Hovered { .. });
    checkbox::Style {
        background: Background::Color(if done {
            theme::GREEN
        } else if hovered {
            theme::SURFACE1
        } else {
            theme::MANTLE
        }),
        icon_color: theme::BASE,
        border: Border {
            radius: 6.0.into(),
            width: 1.5,
            color: accent,
        },
        text_color: Some(theme::TEXT),
    }
}
