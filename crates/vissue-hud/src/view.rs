//! iced widgets for the palette. Drawing only; logic lives in [`crate::palette`].

use iced::widget::{column, container, scrollable, text, Space};
use iced::{Element, Fill, Length};

use crate::app::Message;
use crate::palette::Palette;
use crate::theme;

/// Palette face. Hidden state draws nothing so a `Mode::Hidden` frame is empty.
pub fn view(palette: &Palette) -> Element<'_, Message> {
    if !palette.visible() {
        return Space::new().width(0).height(0).into();
    }

    let mut rows = column![].spacing(2);
    for (i, item) in palette.filtered_items().into_iter().enumerate() {
        let mark = if i == palette.selected_index() {
            ">"
        } else {
            " "
        };
        let line = format!(
            "{mark} {}  {}  [{}]  {}",
            item.id, item.state, item.priority, item.title
        );
        let color = if i == palette.selected_index() {
            theme::TEAL
        } else {
            theme::CREAM
        };
        rows = rows.push(text(line).color(color).size(14));
    }

    let mut body = column![
        text(palette.status_line()).size(12).color(theme::CORAL),
        text(format!("filter: {}", palette.query()))
            .size(16)
            .color(theme::CREAM),
        scrollable(rows).height(Length::Fill),
    ]
    .spacing(8);

    if let Some(excerpt) = palette.excerpt() {
        let label = palette
            .detail()
            .map(|d| format!("{}  {}  {}", d.id, d.state, d.title))
            .unwrap_or_else(|| excerpt.id.clone());
        body = body.push(text(label).size(13).color(theme::TEAL));
        body = body.push(
            scrollable(text(excerpt.text.clone()).size(13).color(theme::CREAM))
                .height(Length::Fixed(140.0)),
        );
    }

    if let Some(note) = palette.note_draft() {
        body = body.push(text(format!("note: {note}")).size(14).color(theme::CORAL));
    }

    body = body.push(
        text("enter excerpt   c claim   n note   esc hide")
            .size(12)
            .color(theme::CREAM),
    );

    container(body.padding(14).spacing(8).width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::NAVY)),
            text_color: Some(theme::CREAM),
            ..container::Style::default()
        })
        .into()
}
