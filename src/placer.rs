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

use crate::channel_router::{ChannelLayout, ChannelState};
use crate::circuit::{Circuit, Position2D};

fn get_net_index_in_layout(channel_layout: &ChannelLayout, net: usize) -> Option<usize> {
    channel_layout
        .iter()
        .enumerate()
        .filter(|(_, v)| v == &&ChannelState::Net(net))
        .map(|(p, _)| p)
        .next()
}

pub fn place_gates(channel_layout: &ChannelLayout, circuits: &mut Vec<Circuit>) {
    let mut desired_channel_layout = vec![ChannelState::Free; channel_layout.len()];
    // Place output circuits (right side of the channel).
    // Since we don't know the width of the channel yet, assign temporary
    // coordinates there.

    // First iteration: Step gates where the inputs are aligned with
    // the outputs of the previous step and swap inputs if necessary.
    for circuit in circuits.iter_mut() {
        // If we have two inputs, swap them if they would cause unnecessary wire crossings.
        if circuit.inputs.len() == 2 {
            if !circuit.can_swap_inputs() {
                continue;
            }

            // Check where the inputs are that we need.
            let p1 = get_net_index_in_layout(
                channel_layout,
                circuit.inputs[0].connection.get_net().unwrap(),
            )
            .unwrap();
            let p2 = get_net_index_in_layout(
                channel_layout,
                circuit.inputs[1].connection.get_net().unwrap(),
            )
            .unwrap();

            if p1 > p2 {
                circuit.swap_inputs();
            }
        } else if circuit.inputs.len() == 1 {
            if let Some(req_input) = circuit.inputs[0].connection.get_net() {
                let p = get_net_index_in_layout(channel_layout, req_input)
                    .unwrap_or_else(|| panic!("Could not find net {}", req_input));

                // If the target slot is still available, place ourselves here.
                // Note: This will also only work with 1x1 gates, circuits might
                //       overlap.
                if desired_channel_layout[p].is_free() {
                    circuit.place(Position2D(100_000, p as u32));
                    desired_channel_layout[p] = ChannelState::Net(req_input);
                }
            }
        }
    }

    // Second iteration: Place everything else.
    for circuit in circuits.iter_mut() {
        // Skip circuits that were already placed.
        if circuit.position.is_some() {
            continue;
        }

        // Find free space to place this circuit.
        let space_pattern = vec![ChannelState::Free; circuit.height() as usize];
        let free_pos = desired_channel_layout
            .windows(circuit.height() as usize)
            .position(|window| window == &*space_pattern)
            // TODO: This adds it at the end, there should be more
            //       efficient things to do.
            .unwrap_or_else(|| desired_channel_layout.len());

        circuit.place(Position2D(100_000, free_pos as u32));

        for i in circuit.inputs.iter() {
            let off = i.position.unwrap().1 as usize;
            while off >= desired_channel_layout.len() {
                desired_channel_layout.push(ChannelState::Free);
            }
            assert!(desired_channel_layout[off].is_free());
            desired_channel_layout[off] = i.connection.into();
        }

        // Hack: Make sure to mark the space between the inputs as occupied.
        assert!(circuit.inputs.len() == 1 || circuit.inputs.len() == 2);
        if circuit.inputs.len() == 2 {
            let off = circuit.inputs[0].position.unwrap().1 as usize;
            assert!(desired_channel_layout[off + 1].is_free());
            desired_channel_layout[off + 1] = ChannelState::Occupied;
        }
    }

    #[cfg(debug_assertions)]
    for c in circuits.iter() {
        if c.position.is_none() {
            panic!("Some circuits were not placed");
        }
    }
}
