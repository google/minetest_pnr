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

use crate::{BlockType, CornerOrientation};
macro_rules! TrivialGate {
    ($gatename:ident, $yosys_id:literal, $basic_gate:expr, $inverted:literal) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $gatename;
        impl BasicCircuitDetails for $gatename {
            fn yosys_id(&self) -> &str {
                $yosys_id
            }

            fn get_layout(&self) -> &[BlockType] {
                use BlockType::*;
                if $inverted {
                    &[
                        WireCorner(CornerOrientation::LeftDown),
                        Air,
                        Gate($basic_gate),
                        Gate(MeseconsGate::Not),
                        WireCorner(CornerOrientation::LeftUp),
                        Air,
                    ]
                } else {
                    &[
                        WireCorner(CornerOrientation::LeftDown),
                        Gate($basic_gate),
                        WireCorner(CornerOrientation::LeftUp),
                    ]
                }
            }

            fn width(&self) -> usize {
                if $inverted {
                    2
                } else {
                    1
                }
            }
            fn height(&self) -> usize {
                3
            }
            fn can_swap_input(&self) -> bool {
                true
            }

            fn input_y_offset(&self, idx: usize) -> usize {
                match idx {
                    0 => 0,
                    1 => 2,
                    _ => unreachable!(),
                }
            }

            fn output_y_offset(&self, idx: usize) -> usize {
                match idx {
                    0 => 1,
                    _ => unreachable!(),
                }
            }
        }
    };
}

TrivialGate!(AndGate, "$_AND_", MeseconsGate::And, false);
TrivialGate!(OrGate, "$_OR_", MeseconsGate::Or, false);
TrivialGate!(XorGate, "$_XOR_", MeseconsGate::Xor, false);
TrivialGate!(NandGate, "$_NAND_", MeseconsGate::Nand, false);
TrivialGate!(NorGate, "$_NOR_", MeseconsGate::Nor, false);

//TrivialGate!(NandGate, "$_NAND_", MeseconsGate::AND, true);
//TrivialGate!(NorGate, "$_NOR_", MeseconsGate::OR, true);
TrivialGate!(XnorGate, "$_XNOR_", MeseconsGate::Xor, true);

macro_rules! SingleFieldGate {
    ($gatename:ident, $yosys_id:literal, $field:expr, $has_input:literal) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $gatename;
        impl BasicCircuitDetails for $gatename {
            fn yosys_id(&self) -> &str {
                $yosys_id
            }

            fn get_layout(&self) -> &[BlockType] {
                use BlockType::*;
                &[$field]
            }

            fn width(&self) -> usize {
                1
            }
            fn height(&self) -> usize {
                1
            }
            fn can_swap_input(&self) -> bool {
                true
            }

            fn input_y_offset(&self, idx: usize) -> usize {
                if $has_input {
                    match idx {
                        0 => 0,
                        _ => unreachable!(),
                    }
                } else {
                    unreachable!();
                }
            }

            fn output_y_offset(&self, idx: usize) -> usize {
                match idx {
                    0 => 0,
                    _ => unreachable!(),
                }
            }
        }
    };
}

SingleFieldGate!(NotGate, "$_NOT_", Gate(MeseconsGate::Not), true);
SingleFieldGate!(InputGate, "INVALID", Gate(MeseconsGate::Input), false);
SingleFieldGate!(OutputGate, "INVALID", Gate(MeseconsGate::Output), true);
SingleFieldGate!(ForwardGate, "INVALID", Gate(MeseconsGate::Forward), true);

macro_rules! SthNotGate {
    ($gatename:ident, $yosys_id:literal, $basic_gate:expr) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $gatename;
        impl BasicCircuitDetails for $gatename {
            fn yosys_id(&self) -> &str {
                $yosys_id
            }

            fn get_layout(&self) -> &[BlockType] {
                use BlockType::*;
                &[
                    WireH,
                    WireCorner(CornerOrientation::LeftDown),
                    Air,
                    Gate($basic_gate),
                    Gate(MeseconsGate::Not),
                    WireCorner(CornerOrientation::LeftUp),
                ]
            }

            fn width(&self) -> usize {
                2
            }
            fn height(&self) -> usize {
                3
            }
            fn can_swap_input(&self) -> bool {
                false
            }

            fn input_y_offset(&self, idx: usize) -> usize {
                match idx {
                    0 => 0,
                    1 => 2,
                    _ => unreachable!(),
                }
            }

            fn output_y_offset(&self, idx: usize) -> usize {
                match idx {
                    0 => 1,
                    _ => unreachable!(),
                }
            }
        }
    };
}
SthNotGate!(AndNotGate, "$_ANDNOT_", MeseconsGate::And);
SthNotGate!(OrNotGate, "$_ORNOT_", MeseconsGate::Or);
