use std::{collections::HashMap, ops::Add, slice::Windows, sync::Arc};

use crate::neural_network::{Action, Location, NeuralNetwork, NeuralTick};
use rayon::prelude::*;

pub mod neural_network;

fn main() {
    let settings = WorldSettings {
        creature_generation_rate: 100,
        food_regen_rate: 12,
    };

    let mut world = World::new(1000, 1000, Tile::Lava, settings);
    println!("World created!");

    loop {
        for _ in 0..100 {
            world.tick();
        }
        println!(
            r#"
            =========={}==========
            Creatures: {}
            "#,
            world.current_tick,
            world.creatures.len()
        )
    }
}

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    fn north(&self, amount: usize) -> Self {
        Self {
            x: self.x,
            y: self.y.saturating_sub(amount),
        }
    }

    fn south(&self, amount: usize) -> Self {
        Self {
            x: self.x,
            y: self.y.saturating_add(amount),
        }
    }

    fn east(&self, amount: usize) -> Self {
        Self {
            x: self.x.saturating_add(amount),
            y: self.y,
        }
    }

    fn west(&self, amount: usize) -> Self {
        Self {
            x: self.x.saturating_sub(amount),
            y: self.y,
        }
    }

    fn cardinal(&self, location: CardinalDirection, amount: usize) -> Self {
        match location {
            CardinalDirection::North => self.north(amount),
            CardinalDirection::South => self.south(amount),
            CardinalDirection::East => self.east(amount),
            CardinalDirection::West => self.west(amount),
        }
    }

    fn randomize(width: usize, height: usize) -> Self {
        Self {
            x: fastrand::usize(0..width),
            y: fastrand::usize(0..height),
        }
    }
}

impl Add for Position {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x.saturating_add(rhs.x),
            y: self.y.saturating_add(rhs.y),
        }
    }
}

pub struct World {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
    creatures: HashMap<Position, Creature>,
    current_tick: u64,
    settings: WorldSettings,
}

#[derive(Clone)]
pub struct WorldSettings {
    food_regen_rate: u8,
    creature_generation_rate: u8,
}

impl World {
    fn new(width: usize, height: usize, border: Tile, settings: WorldSettings) -> Self {
        let mut tiles = vec![Tile::default(); width * height];

        for x in 0..width {
            tiles[x] = border.clone();
            tiles[width * (height - 1) + x] = border.clone();
        }

        for y in 1..height - 1 {
            tiles[y * width] = border.clone();
            tiles[y * width + width - 1] = border.clone();
        }

        let creatures = HashMap::new();
        let current_tick = 0;

        World {
            width,
            height,
            tiles,
            creatures,
            current_tick,
            settings,
        }
    }

    fn get_tile(&self, position: &Position) -> Option<&Tile> {
        if self.check_bounds(position) {
            Some(&self.tiles[position.y * self.width + position.x])
        } else {
            None
        }
    }

    fn check_bounds(&self, position: &Position) -> bool {
        position.x < self.width && position.y < self.height
    }

    fn tick(&mut self) {
        self.current_tick += 1;
        let actions = self
            .creatures
            .par_iter()
            .filter_map(|(position, creature)| {
                if let Some(brain) = &creature.brain {
                    let mut neural_tick = NeuralTick::seed(brain, creature, position, &self);
                    let action = neural_tick.calculate_action(brain);

                    Some((position.clone(), action))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        // for (position, creature) in self.creatures.iter() {
        //     if let Some(brain) = &creature.brain {
        //         let mut neural_tick = NeuralTick::seed(brain, creature, position, &self);
        //         let action = neural_tick.calculate_action(brain);
        //         actions.push((position.clone(), action));
        //     } else {
        //         actions.push((position.clone(), Action::Idle));
        //     }
        // }

        for (position, action) in actions {
            self.apply_action(&position, action);
        }

        for _ in 0..self.settings.creature_generation_rate {
            self.randomize_creature();
        }

        for _ in 0..self.settings.food_regen_rate {
            self.regenerate_food();
        }
    }

    fn apply_action(&mut self, position: &Position, action: Action) {
        {
            let creature = self
                .creatures
                .get_mut(position)
                .expect("An action needs to execute on a position with a creature");

            let energy_cost = action.energy_cost();
            if creature.energy >= energy_cost {
                creature.energy = creature.energy.saturating_sub(action.energy_cost());
            } else {
                self.kill_creature(position);
                return;
            }

            match action {
                Action::Idle => (),
                Action::Eat => {
                    let tile = &mut self.tiles[position.x + position.y * self.width];
                    if let Tile::Ground(data) = tile {
                        if data.food_1 {
                            data.food_1 = false;
                            creature.energy += 50;
                        }
                    }
                }
                Action::Move(location) => {
                    let new_position = creature.relative_position(position, location);
                    if self.check_bounds(&new_position) {
                        self.move_creature(position, new_position);
                    }
                }
                Action::Rotate(rotation) => {
                    creature.rotation.rotate(rotation);
                }
                Action::CreateMembrane(location) => {
                    let spawn_position = creature.relative_position(position, location);
                    let rotation = creature.rotation.clone();
                    creature.offspring += 1;
                    if creature.offspring > 2 {
                        println!("Creature has too many offspring!");
                    }
                    self.spawn_creature(spawn_position, rotation, None);
                }
                Action::CopyDna(location) => {
                    let copy_position = creature.relative_position(position, location);
                    self.copy_dna(position, &copy_position);
                }
            }
        }
    }

    fn move_creature(&mut self, old_position: &Position, new_position: Position) {
        if self.creatures.contains_key(&new_position) {
            return;
        }
        if let Some(creature) = self.creatures.remove(&old_position) {
            let tile = self
                .get_tile(&new_position)
                .expect("Coordinate should be correct");
            if !tile.can_contain_creature() {
                // println!("Creature died to terrain");
                return;
            }
            self.creatures.insert(new_position, creature);
        }
    }

    fn spawn_creature(
        &mut self,
        position: Position,
        rotation: CardinalDirection,
        brain: Option<Arc<NeuralNetwork>>,
    ) {
        let tile = self
            .get_tile(&position)
            .expect("Coordinate should be correct");
        if !tile.can_contain_creature() {
            return;
        }
        self.creatures.entry(position).or_insert_with(|| {
            let creature = Creature::new(self.current_tick, rotation, brain);
            creature
        });
    }

    fn copy_dna(&mut self, old_position: &Position, new_position: &Position) {
        let [source, destination] = self
            .creatures
            .get_disjoint_mut([old_position, new_position]);
        if let Some((source, destination)) = source.zip(destination) {
            destination.brain = source.brain.clone();
        }
    }

    fn kill_creature(&mut self, position: &Position) {
        self.creatures.remove(position);
    }

    fn randomize_creature(&mut self) {
        let position = Position::randomize(self.width, self.height);
        let rotation = CardinalDirection::randomize();
        let brain = Some(Arc::new(NeuralNetwork::randomize()));
        self.spawn_creature(position, rotation, brain);
    }

    fn regenerate_food(&mut self) {
        let position = Position::randomize(self.width, self.height);
        match &mut self.tiles[position.x + position.y * self.width] {
            Tile::Ground(data) => {
                data.food_1 = true;
            }
            Tile::Lava => {}
        }
    }
}

#[derive(Clone)]
pub enum Tile {
    Ground(AccessableTileData),
    Lava,
}

impl Tile {
    fn default() -> Self {
        Tile::Ground(AccessableTileData::default())
    }

    fn can_contain_creature(&self) -> bool {
        match self {
            Tile::Ground(_) => true,
            Tile::Lava => false,
        }
    }

    fn color(&self) -> Color {
        match self {
            Tile::Ground(data) => data.color(),
            Tile::Lava => Color {
                r: 255,
                g: 128,
                b: 0,
            },
        }
    }
}

#[derive(Clone)]
pub struct AccessableTileData {
    food_1: bool,
}

impl AccessableTileData {
    fn default() -> Self {
        AccessableTileData { food_1: false }
    }

    fn color(&self) -> Color {
        if self.food_1 {
            Color { r: 0, g: 255, b: 0 }
        } else {
            Color { r: 0, g: 0, b: 0 }
        }
    }
}

#[derive(PartialEq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}
impl Color {
    fn randomize() -> Color {
        Color {
            r: fastrand::u8(0..255),
            g: fastrand::u8(0..255),
            b: fastrand::u8(0..255),
        }
    }
}

impl PartialOrd for Color {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.r > other.r && self.g > other.g && self.b > other.b {
            Some(std::cmp::Ordering::Greater)
        } else {
            None
        }
    }
}

pub struct Creature {
    born: u64,
    energy: u16,
    rotation: CardinalDirection,
    brain: Option<Arc<NeuralNetwork>>,
    offspring: u64,
}

const INITIAL_CREATURE_ENERGY: u16 = 100;

impl Creature {
    fn new(born: u64, rotation: CardinalDirection, brain: Option<Arc<NeuralNetwork>>) -> Self {
        Creature {
            born,
            energy: INITIAL_CREATURE_ENERGY,
            rotation,
            brain,
            offspring: 0,
        }
    }

    fn relative_position(&self, position: &Position, location: Location) -> Position {
        let cardinal = location.to_cardinal(&self.rotation);
        position.cardinal(cardinal, 1)
    }
}

#[derive(Clone)]
pub enum CardinalDirection {
    North,
    East,
    South,
    West,
}

impl CardinalDirection {
    fn rotate(&mut self, rotation: neural_network::Rotation) {
        match rotation {
            neural_network::Rotation::Clockwise => {
                *self = match self {
                    CardinalDirection::North => CardinalDirection::East,
                    CardinalDirection::East => CardinalDirection::South,
                    CardinalDirection::South => CardinalDirection::West,
                    CardinalDirection::West => CardinalDirection::North,
                };
            }
            neural_network::Rotation::CounterClockwise => {
                *self = match self {
                    CardinalDirection::North => CardinalDirection::West,
                    CardinalDirection::East => CardinalDirection::North,
                    CardinalDirection::South => CardinalDirection::East,
                    CardinalDirection::West => CardinalDirection::South,
                };
            }
        }
    }

    fn randomize() -> CardinalDirection {
        match fastrand::u8(0..4) {
            0 => CardinalDirection::North,
            1 => CardinalDirection::East,
            2 => CardinalDirection::South,
            _ => CardinalDirection::West,
        }
    }
}
