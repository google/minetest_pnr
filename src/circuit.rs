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

use crate::canvas::{BlockType, Canvas, CornerOrientation};
use crate::channel_router::{ChannelLayout, ChannelState};
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
        match self {
            PortConnection::Constant(_) => true,
            _ => false,
        }
    }

    pub fn get_net(&self) -> Option<usize> {
        match self {
            PortConnection::Net(n) => Some(*n),
            _ => None,
        }
    }
}

impl Into<ChannelState> for PortConnection {
    fn into(self) -> ChannelState {
        match self {
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CircuitType {
    INPUT,
    OUTPUT,
    FORWARD,

    AND,
    NAND,
    ANDNOT,
    OR,
    NOR,
    ORNOT,
    NOT,
    XNOR,
    XOR,
}

impl CircuitType {
    pub fn from_verilog(s: &str) -> Self {
        match s {
            "$_ANDNOT_" => CircuitType::ANDNOT, // A & (~B)
            "$_NAND_" => CircuitType::NAND,     // and but inverted
            "$_NOR_" => CircuitType::NOR,       // or but inverted
            "$_OR_" => CircuitType::OR,
            "$_AND_" => CircuitType::AND,
            "$_ORNOT_" => CircuitType::ORNOT, // A | (~B)
            "$_XNOR_" => CircuitType::XNOR,   // ~(XOR)
            "$_XOR_" => CircuitType::XOR,     // XOR
            "$_NOT_" => CircuitType::NOT,
            _ => panic!(
                "Unsupported gate type '{}', make sure to run an abc pass",
                s
            ),
        }
    }

    pub fn mesecon_id(self) -> &'static str {
        use self::CircuitType::*;
        match self {
            INPUT => "mesecons_walllever:wall_lever_off",
            OUTPUT => "mesecons_lamp:lamp_off",
            FORWARD => "mesecons_insulated:insulated_off",
            OR => "mesecons_gates:or_off",
            XOR => "mesecons_gates:xor_off",
            AND => "mesecons_gates:and_off",
            NAND => "mesecons_gates:nand_off",
            NOR => "mesecons_gates:nor_off",
            NOT => "mesecons_gates:not_off",
            _ => panic!("{:?} not a mesecon thing", &self),
        }
    }
}

pub trait CircuitT {
    fn get_type(&self) -> CircuitType;

    fn width(&self) -> u32 {
        use self::CircuitType::*;
        match self.get_type() {
            INPUT | NOT | OUTPUT | FORWARD => 1,
            AND | OR | XOR | NAND | ANDNOT | NOR | ORNOT | XNOR => 2,
        }
    }

    fn height(&self) -> u32 {
        use self::CircuitType::*;
        match self.get_type() {
            INPUT | NOT | OUTPUT | FORWARD => 1,
            AND | OR | XOR | NAND | ANDNOT | NOR | ORNOT | XNOR => {
                assert!(self.inputs().len() == 2 || self.inputs().len() == 1);
                3
            }
        }
    }

    fn inputs(&self) -> &Vec<Port>;
    fn outputs(&self) -> &Vec<Port>;

    fn input_positions(&self) -> Vec<(Position2D, Port)>;
    fn reposition(&mut self, px: u32);
    fn draw(&self, canvas: &mut Canvas);
    fn place(&mut self, position: Position2D);
}

#[derive(Debug)]
pub struct Circuit {
    pub circuit_type: CircuitType,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,

    pub position: Option<Position2D>,
}

impl Circuit {
    pub fn new_external_input_pin(net: Port) -> Self {
        Self {
            circuit_type: CircuitType::INPUT,
            inputs: Vec::new(),
            outputs: vec![net],
            position: None,
        }
    }

    pub fn new_external_output_pin(net: Port) -> Self {
        Self {
            circuit_type: CircuitType::OUTPUT,
            inputs: vec![net],
            outputs: Vec::new(),
            position: None,
        }
    }

    pub fn new_forwarding_pin(net: Port) -> Self {
        Self {
            circuit_type: CircuitType::FORWARD,
            inputs: vec![net],
            outputs: vec![net],
            position: None,
        }
    }

    pub fn width(&self) -> u32 {
        use self::CircuitType::*;
        match self.circuit_type {
            INPUT | NOT | OUTPUT | FORWARD => 1,
            AND | OR | XOR | NAND | ANDNOT | NOR | ORNOT | XNOR => 2,
        }
    }

    pub fn height(&self) -> u32 {
        use self::CircuitType::*;
        match self.circuit_type {
            INPUT | NOT | OUTPUT | FORWARD => 1,
            AND | OR | XOR | NAND | ANDNOT | NOR | ORNOT | XNOR => {
                assert!(self.inputs.len() == 2 || self.inputs.len() == 1);
                3
            }
        }
    }

    pub fn can_swap_inputs(&self) -> bool {
        use self::CircuitType::*;
        match self.circuit_type {
            INPUT | NOT | OUTPUT | FORWARD | ANDNOT | ORNOT => false,
            AND | OR | XOR | NAND | NOR | XNOR => {
                assert!(self.inputs.len() == 2);
                true
            }
        }
    }

    pub fn swap_inputs(&mut self) {
        assert!(self.can_swap_inputs());
        self.inputs.swap(0, 1);
    }

    pub fn reposition(&mut self, px: u32) {
        assert!(self.position.is_some());
        if let Some(ref mut p) = self.position {
            p.0 = px + 1;
        }
        for i in self.inputs.iter_mut() {
            if let Some(ref mut p) = i.position {
                p.0 = px;
            }
        }
        let w = self.width();
        for i in self.outputs.iter_mut() {
            if let Some(ref mut p) = i.position {
                p.0 = px + w + 1;
            }
        }
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        if self.circuit_type == CircuitType::INPUT
            || self.circuit_type == CircuitType::NOT
            || self.circuit_type == CircuitType::OUTPUT
            || self.circuit_type == CircuitType::FORWARD
        {
            // Nothing to do.
        } else {
            // We need to do the input/output wiring.
            let p = self.position.unwrap();
            let w = self.width();

            // Input
            canvas.set(
                p.0 as usize,
                p.1 as usize - 1,
                BlockType::WireCorner(CornerOrientation::LeftDown),
            );
            canvas.set(
                p.0 as usize,
                p.1 as usize + 1,
                BlockType::WireCorner(CornerOrientation::LeftUp),
            );

            // Output:
            canvas.set(
                p.0 as usize + w as usize - 1,
                p.1 as usize,
                BlockType::WireH,
            );

            // XNOR / ORNOT / ANDNOT are not mesecon things and need to be monkeypatched.
            let invert_second = match self.circuit_type {
                CircuitType::XNOR => {
                    canvas.set(
                        p.0 as usize,
                        p.1 as usize,
                        BlockType::Gate(CircuitType::XOR),
                    );
                    canvas.set(
                        p.0 as usize + w as usize - 1,
                        p.1 as usize,
                        BlockType::Gate(CircuitType::NOT),
                    );
                    false
                }
                CircuitType::ORNOT => {
                    canvas.set(p.0 as usize, p.1 as usize, BlockType::Gate(CircuitType::OR));
                    true
                }
                CircuitType::ANDNOT => {
                    canvas.set(
                        p.0 as usize,
                        p.1 as usize,
                        BlockType::Gate(CircuitType::AND),
                    );
                    true
                }
                _ => false,
            };
            if invert_second {
                // Invert second input.
                canvas.set(
                    p.0 as usize - 1,
                    p.1 as usize + 1,
                    BlockType::Gate(CircuitType::NOT),
                );
            }
        }
    }

    pub fn place(&mut self, position: Position2D) {
        if self.position.is_some() {
            // This was already placed.
            panic!("Circuit was already placed");
        }
        self.position = Some(Position2D(position.0 + 1, position.1 + self.height() / 2));
        let mut off_y = 0;
        for i in self.inputs.iter_mut() {
            i.position = Some(Position2D(position.0, position.1 + off_y));
            off_y += 2;
        }
        let w = self.width();
        if !self.outputs.is_empty() {
            assert!(self.outputs.len() == 1);
            if self.circuit_type == CircuitType::FORWARD
                || self.circuit_type == CircuitType::INPUT
                || self.circuit_type == CircuitType::NOT
            {
                self.outputs[0].position = Some(Position2D(position.0 + w + 1, position.1));
            } else {
                self.outputs[0].position = Some(Position2D(position.0 + w + 2, position.1 + 1));
            }
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
            circuit_type: CircuitType::from_verilog(&cell.cell_type),
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
