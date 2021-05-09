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

use serde::{Deserialize, Serialize};
use serde_json::Value;

// Structured - at least a little.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum YosysJsonPortDirection {
    #[serde(rename = "input")]
    Input,
    #[serde(rename = "output")]
    Output,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YosysJsonPort {
    pub direction: YosysJsonPortDirection,
    pub bits: Vec<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YosysJsonCell {
    hide_name: usize,
    #[serde(rename = "type")]
    pub cell_type: String,
    parameters: Value, // can be ignored
    attributes: Value, // can be ignored
    pub connections: std::collections::HashMap<String, Vec<Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YosysJsonModule {
    attributes: Value,
    pub ports: std::collections::HashMap<String, YosysJsonPort>,
    pub cells: std::collections::HashMap<String, YosysJsonCell>,
    netnames: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YosysJson {
    creator: String,
    pub modules: std::collections::HashMap<String, YosysJsonModule>,
}
