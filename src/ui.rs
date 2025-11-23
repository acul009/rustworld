use core::task;
use std::{
    sync::mpsc::channel,
    time::{Duration, Instant},
};

use iced::{
    Background, Border, Element, Length, Subscription, Task,
    advanced::subscription,
    border, event,
    time::{self, every},
    widget::{
        column, container,
        image::{self, viewer},
        text,
    },
    window::{events, open_events},
};
use tokio_stream::wrappers::ReceiverStream;

use crate::simulation::{Snapshot, SnapshotStats, Tile, World, WorldSettings};

pub enum Message {
    UpdateUi(Snapshot),
}

pub struct UI {
    image: image::Handle,
    width: u32,
    height: u32,
    stats: SnapshotStats,
}

impl UI {
    pub fn boot() -> (Self, Task<Message>) {
        let width = 1000u32;
        let height = 1000u32;
        let pixels = vec![255; width as usize * height as usize * 4];
        let image = image::Handle::from_rgba(width, height, pixels);

        let ui = Self {
            image,
            width,
            height,
            stats: SnapshotStats::default(),
        };

        let (send, recv) = tokio::sync::mpsc::channel(1);

        std::thread::spawn(move || {
            let settings = WorldSettings {
                creature_generation_rate: 100,
                food_regen_rate: 40,
            };

            let mut world = World::new(1000, 1000, Tile::Lava, settings);
            let mut last_image = Instant::now();

            loop {
                while last_image.elapsed().as_millis() < 500 {
                    world.tick();
                }
                last_image = Instant::now();

                let snapshot = world.snapshot();

                send.blocking_send(Message::UpdateUi(snapshot)).unwrap();
            }
        });

        (ui, Task::stream(ReceiverStream::new(recv)))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UpdateUi(snapshot) => {
                self.stats = snapshot.stats;
                self.image = image::Handle::from_rgba(self.width, self.height, snapshot.image);

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let viewer = viewer(self.image.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .filter_method(image::FilterMethod::Nearest);

        column![
            text!("Current Tick: {}", self.stats.current_tick),
            text!("Creatures_alive: {}", self.stats.creature_count),
            text!("Max brain count: {}", self.stats.max_brain_count),
            container(viewer).style(|_| container::Style {
                background: Some(Background::Color(iced::Color::BLACK)),
                border: border::color(iced::Color::from_rgb8(255, 0, 0)),
                ..Default::default()
            })
        ]
        .into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        Subscription::none()
        // Subscription::batch([every(Duration::from_millis(1000)).map(|_| Message::WorldTick)])
    }
}
