// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate itertools;
use itertools::Itertools;

use crate::canvas::Canvas;
use crate::channel_router::{ChannelLayout, ChannelState};
use crate::gate::*;
use crate::loader::{YosysJsonCell, YosysJsonPortDirection};

use serde_json::Value;
use std::convert::TryFrom;

#[derive(PartialEq, Debug)]
pub enum IOType {
    Input,
    Output,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position2D(pub u32, pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortConnection {
    Net(usize),
    Constant(bool),
}

impl PortConnection {
    pub fn is_constant(&self) -> bool {
        matches!(self, PortConnection::Constant(_))
    }

    pub fn get_net(&self) -> Option<usize> {
        match self {
            PortConnection::Net(n) => Some(*n),
            _ => None,
        }
    }
}

impl From<PortConnection> for ChannelState {
    fn from(other: PortConnection) -> Self {
        match other {
            PortConnection::Net(n) => ChannelState::Net(n),
            PortConnection::Constant(false) => ChannelState::Occupied,
            PortConnection::Constant(true) => ChannelState::Constant,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq)]
pub struct Port {
    pub connection: PortConnection,
    pub position: Option<Position2D>,
}

impl Port {
    pub fn new_unplaced(connection: PortConnection) -> Self {
        Self {
            connection,
            position: None,
        }
    }

    pub fn from(s: &Value) -> Self {
        let connection = match s {
            Value::Number(n) => PortConnection::Net(n.as_u64().unwrap() as usize),
            Value::String(ref s) => match s.as_ref() {
                "0" | "x" | "y" => PortConnection::Constant(false),
                "1" => PortConnection::Constant(true),
                _ => panic!("Unknown constant value '{}'", s),
            },
            _ => unreachable!(),
        };

        Self {
            connection,
            position: None,
        }
    }
}

impl std::cmp::Ord for Port {
    fn cmp(&self, other: &Port) -> std::cmp::Ordering {
        use PortConnection::*;
        match (self.connection, other.connection) {
            (Net(a), Net(b)) => a.cmp(&b),
            (_, _) => std::cmp::Ordering::Equal,
        }
    }
}

#[derive(Debug)]
pub struct Circuit {
    pub basic_circuit: BasicCircuitYada,

    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,

    pub position: Option<Position2D>,
}

impl Circuit {
    pub fn new_external_input_pin(net: Port) -> Self {
        Self {
            basic_circuit: BasicCircuitYada::input(),
            inputs: Vec::new(),
            outputs: vec![net],
            position: None,
        }
    }

    pub fn new_external_output_pin(net: Port) -> Self {
        Self {
            basic_circuit: BasicCircuitYada::output(),
            inputs: vec![net],
            outputs: Vec::new(),
            position: None,
        }
    }

    pub fn new_forwarding_pin(net: Port) -> Self {
        Self {
            basic_circuit: BasicCircuitYada::forward(),
            inputs: vec![net],
            outputs: vec![net],
            position: None,
        }
    }

    pub fn width(&self) -> u32 {
        self.basic_circuit.width() as u32
    }

    pub fn height(&self) -> u32 {
        self.basic_circuit.height() as u32
    }

    pub fn can_swap_inputs(&self) -> bool {
        self.basic_circuit.can_swap_input()
    }

    pub fn swap_inputs(&mut self) {
        assert!(self.can_swap_inputs());
        self.inputs.swap(0, 1);
    }

    pub fn reposition(&mut self, px: u32) {
        assert!(self.position.is_some());
        if let Some(ref mut p) = self.position {
            p.0 = px;
        }

        let position = self.position.unwrap();

        for (idx, i) in self.inputs.iter_mut().enumerate() {
            i.position = Some(Position2D(
                position.0 - 1,
                position.1 + self.basic_circuit.input_y_offset(idx) as u32,
            ));
        }

        let w = self.width();
        for (idx, i) in self.outputs.iter_mut().enumerate() {
            i.position = Some(Position2D(
                position.0 + w,
                position.1 + self.basic_circuit.output_y_offset(idx) as u32,
            ));
        }
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        let p = self.position.unwrap();
        let w = self.basic_circuit.width();
        let h = self.basic_circuit.height();

        for x in 0..w {
            for y in 0..h {
                canvas.set(
                    p.0 as usize + x,
                    p.1 as usize + y,
                    self.basic_circuit.get_layout()[y * w + x],
                );
            }
        }
    }

    pub fn place(&mut self, position: Position2D) {
        if self.position.is_some() {
            // This was already placed.
            panic!("Circuit was already placed");
        }
        self.position = Some(Position2D(position.0 + 1, position.1));

        for (idx, i) in self.inputs.iter_mut().enumerate() {
            i.position = Some(Position2D(
                position.0 - 1,
                position.1 + self.basic_circuit.input_y_offset(idx) as u32,
            ));
        }

        let w = self.width();
        for (idx, i) in self.outputs.iter_mut().enumerate() {
            i.position = Some(Position2D(
                position.0 + w,
                position.1 + self.basic_circuit.output_y_offset(idx) as u32,
            ));
        }
    }
}

impl TryFrom<&YosysJsonCell> for Circuit {
    type Error = ();
    fn try_from(cell: &YosysJsonCell) -> Result<Self, Self::Error> {
        for i in cell.connections.iter() {
            assert!(i.1.len() == 1);
        }
        let inputs: Vec<_> = cell
            .connections
            .iter()
            .filter(|(k, _)| cell.port_directions[*k] == YosysJsonPortDirection::Input)
            .sorted_by(|(k0, _), (k1, _)| k0.cmp(k1))
            .map(|(_, v)| Port::from(&v[0]))
            .collect();

        let outputs: Vec<_> = cell
            .connections
            .iter()
            .filter(|(k, _)| cell.port_directions[*k] == YosysJsonPortDirection::Output)
            .map(|(_, v)| Port::from(&v[0]))
            .collect();

        Ok(Self {
            basic_circuit: BasicCircuitYada::try_from(&*cell.cell_type).unwrap(),
            inputs,
            outputs,
            position: None,
        })
    }
}

// Determine the channel layout for the given list of circuits.
pub fn determine_channel_layout<'a, T: Iterator<Item = &'a Circuit>>(
    circuits: T,
    io: IOType,
) -> Box<ChannelLayout> {
    let mut res = Vec::new();
    for c in circuits {
        let it = match io {
            IOType::Input => c.inputs.iter(),
            IOType::Output => c.outputs.iter(),
        };
        for i in it {
            let off = i.position.unwrap().1 as usize;
            while off >= res.len() {
                res.push(ChannelState::Occupied);
            }
            res[off] = i.connection.into();
        }
    }
    res.into_boxed_slice()
}
