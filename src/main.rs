use iced::{
    keyboard::{self, key::Named, Key},
    widget::{button, column, container, text_input},
    Element, Event, Length, Task,
};
use iced_layershell::{
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings, StartMode},
    to_layer_message, Application,
};

fn main() -> Result<(), iced_layershell::Error> {
    let settings = Settings {
        layer_settings: LayerShellSettings {
            size: Some((320, 200)),
            exclusive_zone: 200,
            start_mode: StartMode::Active,
            anchor: Anchor::all(),
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        ..Default::default()
    };

    Wizi::run(settings)
}

#[derive(Debug, Default)]
struct Wizi {
    ssid_input: String,
    ssids: Vec<String>,
    selected_index: usize,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum WiziMessage {
    SsidInputChanged(String),
    SelectNext,
    SelectPrevious,
    ResetSelection,
    Quit,
}

impl Application for Wizi {
    type Executor = iced::executor::Default;
    type Message = WiziMessage;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Task<Self::Message>) {
        let ssids = vec!["Brainjuice".into(), "ExtraStrongBrainjuice".into()];
        let wizi = Wizi {
            ssids,
            ..Default::default()
        };
        (
            wizi,
            // focus_next() is a little hacky because other focusable elements may be added at some point.
            // But for now, since the first one is the SSID input, this works cleanly.
            iced::widget::focus_next(),
        )
    }

    fn namespace(&self) -> String {
        "wizi".into()
    }

    fn update(&mut self, message: Self::Message) -> iced::Task<Self::Message> {
        use WiziMessage::*;

        match message {
            SsidInputChanged(ssid) => {
                self.ssid_input = ssid;
                return Task::done(ResetSelection);
            }
            Quit => return iced::exit(),
            SelectNext => {
                self.selected_index = self.selected_index.saturating_add(1);
            }
            SelectPrevious => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            ResetSelection => {
                self.selected_index = 0;
            }
            _ => {}
        };

        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let filtered_ssids = self.ssids.iter().filter(|ssid| {
            ssid.to_lowercase()
                .contains(&self.ssid_input.to_lowercase()) // TODO: replace with fuzzy find
        });

        let wifi_ssid_input: Element<_> = text_input("enter ssid", &self.ssid_input)
            .on_input(WiziMessage::SsidInputChanged)
            .into();

        let avail_aps = column(filtered_ssids.enumerate().map(|(i, ap)| {
            container(
                button(ap.as_str())
                    .width(Length::Fill)
                    .style(move |theme, status| {
                        if i == self.selected_index {
                            button::primary(theme, status)
                        } else {
                            button::text(theme, status)
                        }
                    }),
            )
            .into()
        }));

        column![wifi_ssid_input, avail_aps].into()
    }

    fn theme(&self) -> Self::Theme {
        iced::theme::Theme::Nord
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::event::listen_with(|event, _status, _id| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => match key {
                Key::Named(Named::Escape) => Some(WiziMessage::Quit),
                Key::Named(Named::ArrowDown) => Some(WiziMessage::SelectNext),
                Key::Named(Named::ArrowUp) => Some(WiziMessage::SelectPrevious),
                _ => None,
            },
            _ => None,
        })
    }
}
