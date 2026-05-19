use iced::widget::{Space, container, row, text};
use iced::{Alignment, Background, Border, Element, Length, Padding};

use crate::app::Message;
use crate::game::CpuMode;
use crate::theme::Palette;

pub fn statusbar(palette: &Palette, cpu: CpuMode, count: usize, scanning: bool) -> Element<'static, Message> {
    let led_color = match cpu {
        CpuMode::Jit => palette.accent,
        CpuMode::Interpreter => palette.text_mute,
    };

    let led = container(text("")).width(6).height(6).style(move |_| container::Style {
        background: Some(Background::Color(led_color)),
        border: Border {
            radius: 3.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    });

    let left = row![led, text(cpu.label()).size(11).color(palette.text_dim)]
        .spacing(8)
        .align_y(Alignment::Center);

    let count_label = if count == 1 {
        "1 game".to_owned()
    } else {
        format!("{count} games")
    };
    let count_text = if scanning {
        format!("Scanning…  ·  {count_label}")
    } else {
        count_label
    };
    let count_color = if scanning { palette.accent } else { palette.text_dim };
    let right = text(count_text).size(11).color(count_color);

    let bg = palette.bg_2;
    let border = palette.border;
    let text_color = palette.text_dim;

    container(
        row![left, Space::new().width(Length::Fill), right]
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(28)
    .padding(Padding::from([0, 12]))
    .style(move |_| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 0.0.into(),
        },
        text_color: Some(text_color),
        ..container::Style::default()
    })
    .into()
}
