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
use crate::BlockType;
use std::convert::TryFrom;

mod basic;
mod dff;
mod mesecons;

pub use basic::*;
pub use mesecons::MeseconsGate;

use dff::DffP;

#[derive(Debug, Copy, Clone)]
pub enum BasicCircuitYada {
    And(AndGate),
    Nand(NandGate),
    AndNot(AndNotGate),
    Or(OrGate),
    Nor(NorGate),
    OrNot(OrNotGate),
    Not(NotGate),
    Xnor(XnorGate),
    Xor(XorGate),

    InputGate(InputGate),
    OutputGate(OutputGate),
    ForwardGate(ForwardGate),
    BufGate(BufGate),

    DffP(DffP),
}

impl BasicCircuitYada {
    pub fn input() -> Self {
        Self::InputGate(InputGate)
    }

    pub fn is_input(&self) -> bool {
        matches!(self, BasicCircuitYada::InputGate(_))
    }

    pub fn output() -> Self {
        Self::OutputGate(OutputGate)
    }

    pub fn forward() -> Self {
        Self::ForwardGate(ForwardGate)
    }

    fn inner(&self) -> &dyn BasicCircuitDetails {
        match self {
            BasicCircuitYada::And(ref x) => x,
            BasicCircuitYada::Nand(ref x) => x,
            BasicCircuitYada::AndNot(ref x) => x,
            BasicCircuitYada::Or(ref x) => x,
            BasicCircuitYada::Nor(ref x) => x,
            BasicCircuitYada::OrNot(ref x) => x,
            BasicCircuitYada::Not(ref x) => x,
            BasicCircuitYada::Xnor(ref x) => x,
            BasicCircuitYada::Xor(ref x) => x,

            BasicCircuitYada::InputGate(ref x) => x,
            BasicCircuitYada::OutputGate(ref x) => x,
            BasicCircuitYada::ForwardGate(ref x) => x,
            BasicCircuitYada::BufGate(ref x) => x,

            BasicCircuitYada::DffP(ref x) => x,
        }
    }
}

impl TryFrom<&str> for BasicCircuitYada {
    type Error = ();

    fn try_from(cell_type: &str) -> Result<Self, Self::Error> {
        let gates: &[Self] = &[
            Self::And(AndGate),
            Self::Nand(NandGate),
            Self::AndNot(AndNotGate),
            Self::Or(OrGate),
            Self::Nor(NorGate),
            Self::OrNot(OrNotGate),
            Self::Not(NotGate),
            Self::Xnor(XnorGate),
            Self::Xor(XorGate),
            Self::DffP(DffP),

            Self::BufGate(BufGate),
        ];

        for gate in gates {
            if gate.yosys_id() == cell_type {
                return Ok(*gate);
            }
        }
        Err(())
    }
}

impl BasicCircuitDetails for BasicCircuitYada {
    fn yosys_id(&self) -> &str {
        self.inner().yosys_id()
    }
    fn get_layout(&self) -> &[BlockType] {
        self.inner().get_layout()
    }
    fn width(&self) -> usize {
        self.inner().width()
    }
    fn height(&self) -> usize {
        self.inner().height()
    }
    fn can_swap_input(&self) -> bool {
        self.inner().can_swap_input()
    }
    fn input_names(&self) -> &[&str] {
        self.inner().input_names()
    }
    fn input_y_offset(&self, idx: usize) -> usize {
        self.inner().input_y_offset(idx)
    }
    fn output_names(&self) -> &[&str] {
        self.inner().output_names()
    }
    fn output_y_offset(&self, idx: usize) -> usize {
        self.inner().output_y_offset(idx)
    }
}

pub trait BasicCircuitDetails {
    fn yosys_id(&self) -> &str;
    fn get_layout(&self) -> &[BlockType];
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn input_names(&self) -> &[&str];
    fn input_y_offset(&self, idx: usize) -> usize;
    fn can_swap_input(&self) -> bool;

    fn output_names(&self) -> &[&str];
    fn output_y_offset(&self, idx: usize) -> usize;
}
