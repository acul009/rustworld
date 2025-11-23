use std::collections::HashSet;

use arrayvec::ArrayVec;

use crate::{CardinalDirection, Color, Creature, INITIAL_CREATURE_ENERGY, Position, World};

#[derive(Clone)]
pub enum Rotation {
    Clockwise,
    CounterClockwise,
}
impl Rotation {
    fn randomize() -> Rotation {
        match fastrand::bool() {
            true => Rotation::Clockwise,
            false => Rotation::CounterClockwise,
        }
    }
}

#[derive(Clone)]
pub enum Action {
    Idle,
    Move(Location),
    Rotate(Rotation),
    Eat,
    CreateMembrane(Location),
    CopyDna(Location),
}

impl Action {
    pub fn energy_cost(&self) -> u16 {
        match self {
            Action::Idle => 1,
            Action::Move(_) => 3,
            Action::Rotate(_) => 2,
            Action::Eat => 2,
            Action::CreateMembrane(_) => INITIAL_CREATURE_ENERGY + 5,
            Action::CopyDna(_) => 10,
        }
    }
}

const NEURON_COUNT: usize = 16;
const MIN_GENERATED_NEURONS: usize = 6;
const CONNECTION_COUNT: usize = 16;

pub struct NeuralNetwork {
    neurons: ArrayVec<Neuron, NEURON_COUNT>,
    connections: ArrayVec<NeuralConnection, CONNECTION_COUNT>,
}

impl NeuralNetwork {
    pub(crate) fn randomize() -> NeuralNetwork {
        let neuron_count = fastrand::usize(MIN_GENERATED_NEURONS..=NEURON_COUNT);
        let mut neurons = ArrayVec::new();
        for _ in 0..neuron_count {
            let neuron = Neuron::randomize();
            neurons.push(neuron);
        }

        let input_neurons = neurons
            .iter()
            .enumerate()
            .filter_map(|(index, neuron)| {
                if neuron.has_input() {
                    Some(index)
                } else {
                    None
                }
            })
            .collect::<ArrayVec<_, NEURON_COUNT>>();

        let output_neurons = neurons
            .iter()
            .enumerate()
            .filter_map(|(index, neuron)| {
                if neuron.has_output() {
                    Some(index)
                } else {
                    None
                }
            })
            .collect::<ArrayVec<_, NEURON_COUNT>>();

        if output_neurons.is_empty() || input_neurons.is_empty() {
            return Self::randomize();
        }

        let mut connections = HashSet::new();
        let min_tries = output_neurons.len();
        let connection_generation_tries =
            fastrand::usize(min_tries..=CONNECTION_COUNT.min(output_neurons.len() * 2));
        for _ in 0..connection_generation_tries {
            let source = fastrand::u8(0..input_neurons.len() as u8);
            let destination = fastrand::u8(0..output_neurons.len() as u8);
            if input_neurons[source as usize] == output_neurons[destination as usize] {
                continue;
            }
            connections.insert(NeuralConnection {
                source,
                destination,
            });
        }

        Self {
            neurons,
            connections: connections.into_iter().collect(),
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
pub struct NeuralConnection {
    source: u8,
    destination: u8,
}

pub enum Neuron {
    Input(InputNeuron),
    Output(Action),
}

impl Neuron {
    fn randomize() -> Self {
        let neuron_type = fastrand::u8(0..=9);
        match neuron_type {
            0 => Self::Input(InputNeuron::AlwaysActive),
            1 => Self::Input(InputNeuron::Random),
            2 => {
                let location = Location::randomize();
                Self::Input(InputNeuron::Feeler(location))
            }
            3 => {
                let location = match fastrand::u8(0..5) {
                    0 => Some(Location::InFront),
                    1 => Some(Location::Left),
                    2 => Some(Location::Right),
                    3 => Some(Location::Behind),
                    4 => None,
                    _ => unreachable!(),
                };

                let color = Color::randomize();
                Self::Input(InputNeuron::Eye(location, color))
            }
            4 => Self::Output(Action::Idle),
            5 => Self::Output(Action::Eat),
            6 => {
                let location = Location::randomize();
                Self::Output(Action::Move(location))
            }
            7 => {
                let rotation = Rotation::randomize();
                Self::Output(Action::Rotate(rotation))
            }
            8 => {
                let location = Location::randomize();
                Self::Output(Action::CreateMembrane(location))
            }
            9 => {
                let location = Location::randomize();
                Self::Output(Action::CopyDna(location))
            }
            _ => unreachable!(),
        }
    }

    fn has_output(&self) -> bool {
        match self {
            Neuron::Input(_) => true,
            Neuron::Output(_) => false,
        }
    }

    fn has_input(&self) -> bool {
        match self {
            Neuron::Input(_) => false,
            Neuron::Output(_) => true,
        }
    }
}

pub enum InputNeuron {
    AlwaysActive,
    Random,
    Feeler(Location),
    Eye(Option<Location>, Color),
}

#[derive(Clone)]
pub enum Location {
    InFront,
    Left,
    Right,
    Behind,
}

impl Location {
    pub fn to_cardinal(&self, look_direction: &CardinalDirection) -> CardinalDirection {
        match (self, look_direction) {
            (Location::InFront, cardinal) => cardinal.clone(),
            (Location::Left, CardinalDirection::North) => CardinalDirection::West,
            (Location::Left, CardinalDirection::West) => CardinalDirection::South,
            (Location::Left, CardinalDirection::South) => CardinalDirection::East,
            (Location::Left, CardinalDirection::East) => CardinalDirection::North,
            (Location::Right, CardinalDirection::North) => CardinalDirection::East,
            (Location::Right, CardinalDirection::East) => CardinalDirection::South,
            (Location::Right, CardinalDirection::South) => CardinalDirection::West,
            (Location::Right, CardinalDirection::West) => CardinalDirection::North,
            (Location::Behind, CardinalDirection::North) => CardinalDirection::South,
            (Location::Behind, CardinalDirection::South) => CardinalDirection::North,
            (Location::Behind, CardinalDirection::West) => CardinalDirection::East,
            (Location::Behind, CardinalDirection::East) => CardinalDirection::West,
        }
    }

    fn randomize() -> Self {
        match fastrand::u8(0..4) {
            0 => Location::InFront,
            1 => Location::Left,
            2 => Location::Right,
            _ => Location::Behind,
        }
    }
}

pub struct NeuralTick {
    neuron_states: ArrayVec<NeuronValue, NEURON_COUNT>,
}

impl NeuralTick {
    pub fn seed(net: &NeuralNetwork, me: &Creature, position: &Position, world: &World) -> Self {
        let mut neuron_states = ArrayVec::new();
        for neuron in &net.neurons {
            let initial_output = match neuron {
                Neuron::Input(input_neuron) => match input_neuron {
                    InputNeuron::AlwaysActive => 1.0,
                    InputNeuron::Random => fastrand::f32(),
                    InputNeuron::Feeler(feel_location) => {
                        let feel_position = me.relative_position(position, feel_location.clone());

                        if world.creatures.contains_key(&feel_position) {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    InputNeuron::Eye(look_location, color) => {
                        let look_position = if let Some(location) = look_location {
                            me.relative_position(position, location.clone())
                        } else {
                            position.clone()
                        };

                        if let Some(tile) = world.get_tile(&look_position) {
                            if &tile.color() >= color { 1.0 } else { 0.0 }
                        } else {
                            0.0
                        }
                    }
                },
                Neuron::Output(_action) => 0.0,
            };

            neuron_states.push(NeuronValue {
                input: 0.0,
                output: initial_output,
            });
        }

        Self { neuron_states }
    }

    pub fn calculate_action(&mut self, net: &NeuralNetwork) -> Action {
        for connection in &net.connections {
            let source_value = self.neuron_states[connection.source as usize].output;
            self.neuron_states[connection.destination as usize].input += source_value;
        }

        let mut action = Action::Idle;
        let mut action_impulse = 0.0;

        for i in 0..self.neuron_states.len() {
            match &net.neurons[i] {
                Neuron::Input(_) => (),
                Neuron::Output(neuron_action) => {
                    let neuron_impulse = self.neuron_states[i].input;
                    if neuron_impulse > action_impulse {
                        action = neuron_action.clone();
                        action_impulse = neuron_impulse;
                    }
                }
            }
        }

        action
    }
}

pub struct NeuronValue {
    input: f32,
    output: f32,
}
