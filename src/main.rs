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

#![feature(drain_filter)]

mod canvas;
mod channel_router;
mod circuit;
mod gate;
mod loader;
mod placer;

extern crate rayon;

use crate::canvas::*;
use crate::channel_router::*;
use crate::circuit::*;
use crate::gate::BasicCircuitDetails;
use crate::loader::*;
use crate::placer::place_gates;
use clap::{App, Arg};
use core::convert::TryFrom;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read; //, CircuitTypeT};

fn resolve_gate_dependencies(
    mut circuits: Vec<Circuit>,
    output_ports: Vec<Port>,
) -> Vec<Vec<Circuit>> {
    let mut gate_hierarchy = Vec::new();
    let mut nets_available = Vec::new();
    let mut required_nets = Vec::new();

    while !circuits.is_empty() {
        // Find circuits where all inputs are satisfied.
        let mut placable_gates: Vec<Circuit> = circuits
            .drain_filter(|c| {
                c.inputs.iter().all(|input| match input.connection {
                    PortConnection::Constant(_) => true,
                    PortConnection::Net(net) => nets_available.contains(&net),
                })
            })
            .collect();

        if placable_gates.is_empty() {
            panic!("Circular dependency detected");
        }

        // Sort gates by output net number.
        placable_gates.sort_by(|a, b| a.outputs[0].cmp(&b.outputs[0]));

        for g in placable_gates.iter() {
            for o in g.outputs.iter() {
                if let PortConnection::Net(net) = o.connection {
                    if !nets_available.contains(&net) {
                        nets_available.push(net);
                    }
                }
            }
            for i in g.inputs.iter() {
                if let PortConnection::Net(net) = i.connection {
                    if !required_nets.contains(&net) {
                        required_nets.push(net);
                    }
                }
            }
        }

        gate_hierarchy.push(placable_gates);
    }

    gate_hierarchy.push(
        output_ports
            .iter()
            .map(|&bit| Circuit::new_external_output_pin(bit))
            .collect(),
    );

    let output_nets = output_ports
        .iter()
        .filter(|n| !n.connection.is_constant())
        .map(|n| match n.connection {
            PortConnection::Net(i) => i,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    for n in nets_available
        .iter()
        .filter(|v| !required_nets.contains(v) && !output_nets.contains(v))
    {
        panic!("[!] Net {} seems to be not used - bug?", n);
    }

    gate_hierarchy
}

fn parse_json(filepath: &str) -> std::io::Result<Vec<Vec<Circuit>>> {
    let mut file = File::open(filepath)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let v: YosysJson = serde_json::from_str(&*buf)?;
    if v.modules.len() != 1 {
        panic!("Input file contained no or more than one module");
    }
    let m = &v.modules.values().next().unwrap();

    // Convert cells to `Circuit`s.
    let mut circuits: Vec<_> = m
        .cells
        .values()
        .map(|v| Circuit::try_from(v).unwrap_or_else(|_| panic!("Could not convert {:?}", v)))
        .collect();

    // Add input connections.
    for bit in m
        .ports
        .iter()
        .map(|p| p.1)
        .filter(|p| p.direction == YosysJsonPortDirection::Input)
        .flat_map(|p| &p.bits)
    {
        circuits.push(Circuit::new_external_input_pin(Port::from(bit)));
    }

    let mut output_pins = m
        .ports
        .iter()
        .map(|p| p.1)
        .filter(|p| p.direction == YosysJsonPortDirection::Output)
        .flat_map(|p| &p.bits)
        .map(|bits| Port::from(&bits))
        .collect::<Vec<_>>();
    output_pins.sort();

    // Let's keep the input layout consistent.
    circuits.sort_by(
        |a, b| match (a.basic_circuit.is_input(), b.basic_circuit.is_input()) {
            (true, true) => a.outputs[0].cmp(&b.outputs[0]),
            (true, false) => std::cmp::Ordering::Less,
            (_, _) => a.inputs[0].cmp(&b.inputs[0]),
        },
    );

    println!("[*] Calculating gate layout.");
    Ok(resolve_gate_dependencies(circuits, output_pins))
}

fn main() -> std::io::Result<()> {
    let parameters = App::new("Minetest HDL")
        .version("0.1")
        .author("Kevin Hamacher <hamacher@google.com>")
        .about("Converts synthesized circuit (yosys json output) to a minetest schematic that can be placed in minetest")
        .arg(
            Arg::with_name("text")
                .short("t")
                .long("text")
                .help("Print text overview on STDOUT"),
        )
        .arg(
            Arg::with_name("write_lua")
                .short("l")
                .long("write_lua")
                .help("Writes a lua blueprint")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("write_mts")
                .short("m")
                .long("write_mts")
                .help("Writes a MTS blueprint (binary format)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1),
        )
        .get_matches();

    let lua_filename = parameters.value_of("write_lua");
    let mts_filename = parameters.value_of("write_mts");

    let mut gate_hierarchy = parse_json(parameters.value_of("INPUT").unwrap())?;

    println!("[*] Adding 'forwarding' gates to keep unused nets.");
    {
        // Starting with 2 here since we will always have all inputs at the 1st
        // stage (0th = input bits for the whole circuitry), so we need to start
        // checking that the 1st stage will 'reexport' the required bits.
        for idx in (2..gate_hierarchy.len()).rev() {
            let required_inputs: Vec<_> = gate_hierarchy[idx]
                .iter()
                .flat_map(|c| c.inputs.iter().map(|i| i.connection))
                .filter(|v| !v.is_constant())
                .map(|v| v.get_net().unwrap())
                .collect();

            let mut outputs_available: Vec<_> = gate_hierarchy[idx - 1]
                .iter()
                .flat_map(|c| c.outputs.iter().map(|o| o.connection))
                .filter(|v| !v.is_constant())
                .map(|v| v.get_net().unwrap())
                .collect();

            for ri in required_inputs {
                if !outputs_available.contains(&ri) {
                    // The previous segment did not provide the required output,
                    // so add a dependency.
                    gate_hierarchy[idx - 1].push(Circuit::new_forwarding_pin(Port::new_unplaced(
                        PortConnection::Net(ri),
                    )));
                    outputs_available.push(ri);
                }
            }
        }
    }

    println!("[*] Performing channel routing.");

    // Place the first circuit block.
    let mut block_x_start = 0;
    {
        let mut gate_y = 0;
        let mut max_w = 0;
        for c in gate_hierarchy[0].iter_mut() {
            c.place(Position2D(block_x_start, gate_y));
            gate_y += c.height();
            if c.width() > max_w {
                max_w = c.width();
            }
        }
        block_x_start += max_w;
    }

    // Go at least a little bit straight before starting the channel router.
    block_x_start += 1;

    // 1) Place
    println!("[*] Placing gates");
    for gategroup_idx in 0..gate_hierarchy.len() - 1 {
        // Get the current input layout (left side of the channel).
        let channel_layout =
            determine_channel_layout(gate_hierarchy[gategroup_idx].iter(), IOType::Output);
        let n_inputs = channel_layout
            .iter()
            .filter(|x| matches!(**x, ChannelState::Net(_)))
            .count();
        println!(
            " [+] Step {}/{} - {} inputs to {} gates",
            gategroup_idx + 1,
            gate_hierarchy.len() - 1,
            n_inputs,
            gate_hierarchy[gategroup_idx + 1].len()
        );

        // Determine required channel layout (input pins of the next group).
        place_gates(&channel_layout, &mut gate_hierarchy[gategroup_idx + 1]);
    }

    println!("[*] Routing");
    let ops_per_step = (0..gate_hierarchy.len() - 1)
        .into_par_iter()
        .map(|gategroup_idx| {
            let channel_layout =
                determine_channel_layout(gate_hierarchy[gategroup_idx].iter(), IOType::Output);
            // Determine required channel layout (input pins of the next group).
            let desired_channel_layout =
                determine_channel_layout(gate_hierarchy[gategroup_idx + 1].iter(), IOType::Input);
            route_channel(&channel_layout, &desired_channel_layout)
        })
        .collect::<Vec<_>>();

    let mut canvas = Canvas::new();
    let mut place_constants_here = Vec::new();
    println!("[*] Drawing to canvas");
    for (gategroup_idx, ops) in ops_per_step.iter().enumerate() {
        let channel_layout =
            determine_channel_layout(gate_hierarchy[gategroup_idx].iter(), IOType::Output);
        // Determine required channel layout (input pins of the next group).
        let desired_channel_layout =
            determine_channel_layout(gate_hierarchy[gategroup_idx + 1].iter(), IOType::Input);

        // Let's draw our channels.
        // 1 pixel initial wires
        const WIRE_LENGTH_AFTER_GATE: usize = 1;
        let mut x = block_x_start;
        for xi in 0..WIRE_LENGTH_AFTER_GATE {
            for (ly, cly) in channel_layout.iter().enumerate() {
                if cly.contains_net() {
                    canvas.set(x as usize + xi, ly as usize, BlockType::WireH);
                }
            }
        }
        x += WIRE_LENGTH_AFTER_GATE as u32;

        canvas.set_channel_wires(&ops, &mut x);

        for c in gate_hierarchy[gategroup_idx + 1].iter_mut() {
            c.reposition(x);
        }

        // Place constant inputs
        for pos in desired_channel_layout
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_constant_on())
            .map(|(idx, _)| idx)
        {
            place_constants_here.push((x as usize, pos));
        }

        let widest_gate = gate_hierarchy[gategroup_idx + 1]
            .iter()
            .map(|x| x.width())
            .max()
            .unwrap_or(1);
        block_x_start = x + widest_gate;
    }

    for gate_group in gate_hierarchy.iter() {
        let widest_gate = gate_group.iter().map(|x| x.width()).max().unwrap_or(1);
        for g in gate_group.iter() {
            let gate_pos = g
                .position
                .unwrap_or_else(|| panic!("Circuit {:#?} was not placed!", g));
            g.draw(&mut canvas);
            for dx in g.width()..widest_gate {
                let p = gate_pos;
                canvas.set(
                    p.0 as usize + dx as usize,
                    p.1 as usize + g.basic_circuit.output_y_offset(0),
                    BlockType::WireH,
                );
            }
        }
    }

    for (x, y) in place_constants_here.iter() {
        canvas.set(*x, *y, BlockType::Constant);
    }

    println!("[*] Canvas dimensions: {:?}", canvas.dimensions());
    if parameters.occurrences_of("text") > 0 {
        println!("*** text overview ***");
        canvas.draw();
    }
    if let Some(f) = lua_filename {
        println!("[*] Generating lua schematic file");
        canvas.generate_lua_schematic(f)?;
    }

    if let Some(f) = mts_filename {
        println!("[*] Generating MTS schematic file");
        canvas.serialize_to_mts(f)?
    }

    Ok(())
}
