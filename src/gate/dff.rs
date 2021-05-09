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
use crate::gate::{BasicCircuitDetails, MeseconsGate};
use crate::{BlockType, CornerOrientation, TRotation};

#[derive(Copy, Clone, Debug)]
pub struct DffP;

impl BasicCircuitDetails for DffP {
    fn yosys_id(&self) -> &str {
        "DFF"
    }

    fn get_layout(&self) -> &[BlockType] {
        use BlockType::*;
        use CornerOrientation::*;
        &[
            WireT(TRotation::LeftRightDown), //WireStar,
            Gate(MeseconsGate::Not),
            WireCorner(LeftDown),
            Air,
            Air,
            Air,
            Air,
            WireV,
            Air,
            Gate(MeseconsGate::And),
            WireH,
            WireCorner(LeftDown),
            Air,
            WireCorner(DownRight),
            WireCrossing,
            WireH,
            WireT(TRotation::LeftUpDown), //WireStar,
            Air,
            Gate(MeseconsGate::Nor),
            WireT(TRotation::LeftRightDown), //WireStar,
            WireCorner(LeftUp),
            WireV,
            Air,
            WireV,
            Air,
            WireCorner(UpRight),
            WireCrossing,
            WireCorner(LeftDown),
            WireV,
            Air,
            WireV,
            Air,
            WireCorner(DownRight),
            WireCorner(LeftUp),
            WireV,
            WireV,
            Air,
            Gate(MeseconsGate::And),
            WireCorner(LeftDown),
            Gate(MeseconsGate::Nor),
            WireH,
            WireCorner(LeftUp),
            WireCorner(UpRight),
            WireH,
            WireCorner(LeftUp),
            WireCorner(UpRight),
            WireCorner(LeftUp),
            Air,
            Air,
        ]
    }

    fn width(&self) -> usize {
        7
    }
    fn height(&self) -> usize {
        7
    }
    fn can_swap_input(&self) -> bool {
        false
    }

    fn input_names(&self) -> &[&str] {
        &["C", "D"]
    }

    fn input_y_offset(&self, idx: usize) -> usize {
        match idx {
            0 => 0,
            1 => 2,
            _ => unreachable!(),
        }
    }

    fn output_names(&self) -> &[&str] {
        &["Q"]
    }

    fn output_y_offset(&self, idx: usize) -> usize {
        match idx {
            0 => 1,
            _ => unreachable!(),
        }
    }
}
