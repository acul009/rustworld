use crate::ui::UI;

pub mod simulation;
pub mod ui;

fn main() {
    iced::application(UI::boot, UI::update, UI::view)
        .subscription(UI::subscription)
        .run()
        .unwrap();

    // let settings = WorldSettings {
    //     creature_generation_rate: 1000,
    //     food_regen_rate: 12,
    // };

    // let mut world = World::new(1000, 1000, Tile::Lava, settings);
    // println!("World created!");
}
