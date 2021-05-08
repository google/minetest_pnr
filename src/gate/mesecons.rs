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

// Mesecons gates
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MeseconsGate {
    Input,
    Output,
    Forward,

    And,
    Nand,
    Or,
    Nor,
    Not,
    Xor,
}

impl MeseconsGate {
    pub fn mesecon_id(self) -> &'static str {
        use self::MeseconsGate::*;
        match self {
            Input => "mesecons_walllever:wall_lever_off",
            Output => "mesecons_lamp:lamp_off",
            Forward => "mesecons_insulated:insulated_off",
            Or => "mesecons_gates:or_off",
            Xor => "mesecons_gates:xor_off",
            And => "mesecons_gates:and_off",
            Nand => "mesecons_gates:nand_off",
            Nor => "mesecons_gates:nor_off",
            Not => "mesecons_gates:not_off",
        }
    }
}
