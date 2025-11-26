use std::{collections::HashMap, ops::Mul, time::Instant};

use iced::{
    Element, Length, Point, Rectangle, Renderer, Size, Subscription, Task, Theme,
    widget::{
        Canvas,
        canvas::{self, Frame},
        column, image, text,
    },
    window,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio_stream::wrappers::ReceiverStream;

use crate::simulation::{Creature, Position, Snapshot, Tile, World, WorldSettings};

#[derive(Debug)]
pub enum Message {
    UpdateUi(Snapshot),
    Tick,
    Allocated(Result<image::Allocation, image::Error>),
}

pub struct UI {
    allocation: Option<image::Allocation>,
    placeholder: image::Handle,
    world: Option<World>,
    snapshot: Snapshot,
    width: u32,
    height: u32,
}

impl UI {
    pub fn boot() -> (Self, Task<Message>) {
        let width = 1000u32;
        let height = 1000u32;
        let pixels = vec![255; width as usize * height as usize * 4];

        let settings = WorldSettings {
            creature_generation_rate: 3,
            food_regen_rate: 30,
        };
        let world = World::new(1000, 1000, Tile::Lava, settings);

        let ui = Self {
            allocation: None,
            world: Some(world),
            snapshot: Snapshot::default(),
            placeholder: image::Handle::from_rgba(width, height, pixels),
            width,
            height,
        };

        // let (send, recv) = tokio::sync::mpsc::channel(1);

        // std::thread::spawn(move || {
        //     let settings = WorldSettings {
        //         creature_generation_rate: 3,
        //         food_regen_rate: 40,
        //     };

        //     let mut world = World::new(1000, 1000, Tile::Lava, settings);
        //     let mut last_image = Instant::now();

        //     loop {
        //         while last_image.elapsed().as_millis() < 16 {
        //             world.tick();
        //         }
        //         last_image = Instant::now();

        //         let snapshot = world.snapshot();

        //         send.blocking_send(Message::UpdateUi(snapshot)).unwrap();
        //     }
        // });

        // (ui, Task::stream(ReceiverStream::new(recv)))
        (ui, Task::done(Message::Tick))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UpdateUi(snapshot) => {
                self.snapshot = snapshot;

                Task::none()
            }
            Message::Tick => {
                if let Some(world) = &mut self.world {
                    world.tick();
                    self.snapshot = world.snapshot();
                }

                // Do not parallelize the image building process - it's slower than single-threaded

                self.snapshot.background_upload().map(Message::Allocated)
            }
            Message::Allocated(result) => match result {
                Err(err) => {
                    eprintln!("{err}");
                    Task::none()
                }
                Ok(allocation) => {
                    self.allocation = Some(allocation);
                    Task::done(Message::Tick)
                }
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let handle = self
            .allocation
            .as_ref()
            .map(|allocation| allocation.handle())
            .unwrap_or_else(|| &self.placeholder);
        column![
            text!("Current Tick: {}", self.snapshot.current_tick()),
            text!("Creatures_alive: {}", self.snapshot.creature_count()),
            image(handle)
                .height(Length::Fill)
                .width(Length::Fill)
                .filter_method(image::FilterMethod::Nearest)
        ]
        .into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        // window::frames().map(|_| Message::Tick)
        Subscription::none()
        // Subscription::batch([every(Duration::from_millis(1000)).map(|_| Message::WorldTick)])
    }
}

pub struct Board<'a> {
    pub width: u32,
    pub height: u32,
    pub creatures: &'a HashMap<Position, Creature>,
    pub background: &'a image::Handle,
}

impl<'a> canvas::Program<Message> for Board<'a> {
    type State = ();

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        cursor: iced::advanced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let tile_width = bounds.width / self.width as f32;
        let tile_height = bounds.height / self.height as f32;
        let tile_len = tile_width.min(tile_height);
        let tile_size = iced::Size::new(tile_len, tile_len);

        let mut frame = Frame::new(
            renderer,
            Size::new(
                tile_width * self.width as f32,
                tile_height * self.height as f32,
            ),
        );
        frame.draw_image(
            Rectangle::new(Point::ORIGIN, bounds.size()),
            self.background,
        );

        vec![frame.into_geometry()]
    }
}
