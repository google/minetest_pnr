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

use crate::channel_router::{ChannelOp, ChannelSubState, WireConnection};
use crate::CircuitType;
use byteorder::{BigEndian, WriteBytesExt};
use deflate::write::ZlibEncoder;
use deflate::Compression;
use std::fs::File;
use std::io::{BufWriter, Write};

const BLOCK_IDS: &[&str; 16] = &[
    "air",
    "stone",
    "mesecons_lamp:lamp_off",
    "mesecons_walllever:wall_lever_off",
    // Gates
    "mesecons_gates:and_off",
    "mesecons_gates:nand_off",
    "mesecons_gates:nor_off",
    "mesecons_gates:not_off",
    "mesecons_gates:or_off",
    "mesecons_gates:xor_off",
    // Regular wire
    "mesecons:mesecon_off",
    // Insulated wires
    "mesecons_insulated:insulated_off",
    "mesecons_extrawires:corner_off",
    "mesecons_extrawires:tjunction_off",
    "mesecons_extrawires:crossover_off",
    // For constant inputs
    "mesecons_torch:mesecon_torch_off",
];

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CornerOrientation {
    LeftUp,
    LeftDown,
    DownRight,
    UpRight,
}

impl CornerOrientation {
    pub fn get_param2(self) -> u8 {
        use self::CornerOrientation::*;
        match self {
            LeftUp => 0,
            LeftDown => 3,
            DownRight => 2,
            UpRight => 1,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TRotation {
    LeftRightDown,
    LeftRightUp,
    RightUpDown,
    LeftUpDown,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BlockType {
    Air,
    WireH,
    WireV,
    WireCrossing,
    WireT(TRotation),
    WireCorner(CornerOrientation),
    WireStar,
    Gate(CircuitType),

    Input,
    Output,
    Constant,
}

impl Default for BlockType {
    fn default() -> Self {
        BlockType::Air
    }
}

impl BlockType {
    pub fn c(self) -> char {
        use self::BlockType::*;
        match self {
            Air => ' ',
            WireH => '─',
            WireV => '│',
            WireCrossing => '╂',
            WireCorner(CornerOrientation::DownRight) => '┌',
            WireCorner(CornerOrientation::LeftUp) => '┘',
            WireCorner(CornerOrientation::LeftDown) => '┐',
            WireCorner(CornerOrientation::UpRight) => '└',
            WireT(TRotation::LeftRightDown) => '┬',
            WireT(TRotation::LeftRightUp) => '┴',
            WireT(TRotation::RightUpDown) => '├',
            WireT(TRotation::LeftUpDown) => '┤',
            WireStar => '┼',

            Gate(CircuitType::INPUT) => '░',
            Gate(CircuitType::FORWARD) => '»',
            Gate(CircuitType::NOT) => '¬',
            Gate(CircuitType::OR) => 'v',
            Gate(CircuitType::AND) => '^',
            Gate(_) => '▓',

            Input => '─',
            Output => '─',
            Constant => 'o',
        }
    }

    pub fn minetest_type(self) -> &'static str {
        use self::BlockType::*;
        match self {
            Air => "air",
            WireH => "mesecons_insulated:insulated_off",
            WireV => "mesecons_insulated:insulated_off",
            WireCrossing => "mesecons_extrawires:crossover_off",
            WireCorner(_) => "mesecons_extrawires:corner_off",
            WireT(_) => "mesecons_extrawires:tjunction_off",
            WireStar => "mesecons:mesecon_off",
            Gate(gate) => gate.mesecon_id(),

            Input => "mesecons_insulated:insulated_off",
            Output => "mesecons_insulated:insulated_off",
            Constant => "mesecons_torch:mesecon_torch_off",
        }
    }

    pub fn get_param2(self) -> u8 {
        use self::BlockType::*;
        match self {
            WireV => 0,
            WireH => 3,

            WireCorner(co) => co.get_param2(),

            WireT(TRotation::LeftUpDown) => 0,
            WireT(TRotation::LeftRightUp) => 1,
            WireT(TRotation::RightUpDown) => 2,
            WireT(TRotation::LeftRightDown) => 3,

            // gate 'input / output' pseudo types (are actually wires)
            Input => 3,
            Output => 3,

            Gate(CircuitType::OUTPUT) => 0,
            Gate(CircuitType::INPUT) => 0,
            Gate(_) => 3,

            // Can be rotated in any way.
            Air => 0,
            WireCrossing => 0,
            WireStar => 0,
            Constant => 0,
        }
    }
}

// Note that they do not need to be bigger than 64 * 1024 as that's a limitation
// of minetest.
const CANVAS_MAX_W: usize = 8 * 1024;
const CANVAS_MAX_H: usize = 8 * 1024;

pub struct Canvas {
    data: Box<[[BlockType; CANVAS_MAX_H]]>,
    width: usize,
    height: usize,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            data: vec![[BlockType::Air; CANVAS_MAX_H]; CANVAS_MAX_W].into_boxed_slice(),
            height: 0,
            width: 0,
        }
    }

    pub fn set(&mut self, x: usize, y: usize, c: BlockType) {
        if x >= std::u16::MAX as _ || y >= std::u16::MAX as _ {
            panic!("Sorry, circuit too large for minetest (MTS format limitation)");
        }
        if x >= CANVAS_MAX_W || y >= CANVAS_MAX_H {
            panic!("Sorry, circuit too large for the internal canvas, consider increasing CANVAS_MAX_{{W/H}}");
        }
        if x > self.width {
            self.width = x;
        }
        if y > self.height {
            self.height = y;
        }
        self.data[x][y] = c;
    }

    pub fn get(&self, x: usize, y: usize) -> BlockType {
        if x >= std::u16::MAX as _ || y >= std::u16::MAX as _ {
            panic!("Sorry, circuit too large for minetest (MTS format limitation)");
        }
        if x >= CANVAS_MAX_W || y >= CANVAS_MAX_H {
            panic!("Sorry, circuit too large for the internal canvas, consider increasing CANVAS_MAX_{{W/H}}");
        }
        self.data[x][y]
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width + 1, self.height + 1)
    }

    pub fn draw(&self) {
        let d = self.dimensions();
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        for line in 0..d.1 {
            for c in 0..d.0 {
                let c = &self.get(c, line).c();
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf);
                let buf = &buf[0..c.len_utf8()];
                lock.write_all(&buf).unwrap();
            }
            lock.write_all(b"\n").unwrap();
        }
    }

    pub fn set_channel_wires(&mut self, ops: &[ChannelSubState], x: &mut u32) {
        // Draw actual channel.
        // Defines how many additional blocks of space should be next to the wires.
        const CHANNEL_WIRE_PADDING: usize = 0;
        for state in ops.iter() {
            for xi in 0..(1 + CHANNEL_WIRE_PADDING) {
                // Go through the current channel state.
                for channel_idx in 0..state.occupancy_map.len() {
                    if state.occupancy_map.get(channel_idx).unwrap() != 0 {
                        // Channel is occupied, draw a wire (or crossing).
                        let x = *x as usize;
                        if self.get(x + xi, channel_idx) == BlockType::WireV {
                            self.set(x + xi, channel_idx, BlockType::WireCrossing);
                        } else {
                            self.set(x + xi, channel_idx, BlockType::WireH);
                        }
                    }
                }
            }

            for op in state.wires.iter() {
                let WireConnection {
                    from: source,
                    to: mut destination,
                    mode: op,
                } = op.clone();

                if op == ChannelOp::Copy {
                    destination.push(source);
                }

                // Connect all destination pins of the same net if there is more than one.
                let mi = *destination.iter().min().unwrap();
                let ma = *destination.iter().max().unwrap();

                if mi != ma {
                    for y in mi..=ma {
                        if y == mi {
                            self.set(
                                *x as _,
                                y as _,
                                BlockType::WireCorner(CornerOrientation::DownRight),
                            );
                        } else if y == ma {
                            self.set(
                                *x as _,
                                y as _,
                                BlockType::WireCorner(CornerOrientation::UpRight),
                            );
                        } else if destination.contains(&y) {
                            self.set(*x as _, y as _, BlockType::WireT(TRotation::RightUpDown));
                        } else if self.get(*x as _, y as _) == BlockType::WireH {
                            self.set(*x as _, y as _, BlockType::WireCrossing);
                        } else if self.get(*x as _, y as _) == BlockType::Air {
                            self.set(*x as _, y as _, BlockType::WireV);
                        }
                    }
                }

                #[derive(Debug, PartialEq, Copy, Clone)]
                enum SourcePosition {
                    Above,
                    Below,
                }

                let (start, end, spos) = if source < mi {
                    (source, mi, SourcePosition::Above)
                } else if source > ma {
                    (ma, source, SourcePosition::Below)
                } else {
                    if destination.contains(&source) {
                        if source == mi {
                            self.set(*x as _, source, BlockType::WireT(TRotation::LeftRightDown));
                        } else if source == ma {
                            self.set(*x as _, source, BlockType::WireT(TRotation::LeftRightUp));
                        } else {
                            self.set(*x as _, source, BlockType::WireStar);
                        }
                    } else {
                        self.set(*x as _, source, BlockType::WireT(TRotation::LeftUpDown));
                    }
                    continue;
                };

                // Connect input to ranges.
                for y in start..=end {
                    if y == start || y == end {
                        let x = *x as usize;
                        let y = y as usize;

                        if y == source && op == ChannelOp::Copy {
                            continue;
                        }

                        let block_type = match (y == source, spos, self.get(x, y)) {
                            // @ Source - should be empty.
                            (true, SourcePosition::Above, BlockType::Air) => {
                                BlockType::WireCorner(CornerOrientation::LeftDown)
                            }
                            (true, SourcePosition::Below, BlockType::Air) => {
                                BlockType::WireCorner(CornerOrientation::LeftUp)
                            }

                            // Destination, should not be empty.
                            // If we're coming from above, we either have already the correct connection (single pin)
                            (
                                false,
                                SourcePosition::Above,
                                BlockType::WireCorner(CornerOrientation::DownRight),
                            ) => BlockType::WireT(TRotation::RightUpDown),
                            (
                                false,
                                SourcePosition::Below,
                                BlockType::WireCorner(CornerOrientation::UpRight),
                            ) => BlockType::WireT(TRotation::RightUpDown),
                            (false, SourcePosition::Below, BlockType::WireH) => {
                                BlockType::WireCorner(CornerOrientation::DownRight)
                            }
                            (false, SourcePosition::Above, BlockType::WireH) => {
                                BlockType::WireCorner(CornerOrientation::UpRight)
                            }
                            (is_start, pos, prev) => {
                                println!(
                                    "Unexpected block type {:?} is_start={} pos={:?} - {}:{}",
                                    prev, is_start, pos, x, y
                                );
                                BlockType::Constant
                            }
                        };
                        self.set(x as _, y as _, block_type);
                    } else if self.get(*x as _, y as _) == BlockType::Air {
                        self.set(*x as _, y as _, BlockType::WireV);
                    } else if self.get(*x as _, y as _) == BlockType::WireH {
                        self.set(*x as _, y as _, BlockType::WireCrossing);
                    }
                }

                if op == ChannelOp::Copy {
                    self.set(
                        *x as _,
                        source as usize,
                        BlockType::WireT(match spos {
                            SourcePosition::Below => TRotation::LeftRightUp,
                            SourcePosition::Above => TRotation::LeftRightDown,
                        }),
                    );
                }
            }
            *x += 1 + CHANNEL_WIRE_PADDING as u32;
        }
    }

    pub fn generate_lua_schematic(&self, fname: &str) -> std::io::Result<()> {
        let d = self.dimensions();
        let mut file = File::create(fname)?;
        file.write_all("schematic = {\n".as_bytes())?;
        file.write_all(format!("\tsize = {{x={}, y=2, z={}}},\n", d.1, d.0).as_bytes())?;
        file.write_all("\tdata = {\n".as_bytes())?;

        for x in 0..d.0 {
            for z in 0..2 {
                for y in 0..d.1 {
                    file.write_all(
                        if z == 0 {
                            "\t\t{name=\"stone\"},\n".to_string()
                        } else {
                            format!(
                                "\t\t{{name=\"{}\", param2=param2}},\n",
                                self.get(x, y).minetest_type()
                            )
                        }
                        .as_bytes(),
                    )?;
                }
            }
        }

        file.write_all("\t}\n".as_bytes())?;
        file.write_all("}\n".as_bytes())?;
        Ok(())
    }

    pub fn serialize_to_mts(&self, fname: &str) -> std::io::Result<()> {
        // Map size.
        let d = self.dimensions();
        // The minetest source is not consistent when it comes to the type of
        // this field (i16 vs u16), so picking the conservative option here.
        if d.1 > std::i16::MAX as _ || d.0 >= std::i16::MAX as _ {
            panic!("Schematic too big to export to a mts :/");
        }

        let mut file = File::create(fname)?;

        file.write_all(b"MTSM")?; // MTSCHEM_FILE_SIGNATURE
        file.write_u16::<BigEndian>(1)?; // Version 1

        file.write_i16::<BigEndian>(d.1 as i16)?;
        file.write_i16::<BigEndian>(2)?;
        file.write_i16::<BigEndian>(d.0 as i16)?;

        // No need to do the prob table as we're totally old.
        // Write # node names.
        file.write_u16::<BigEndian>(BLOCK_IDS.len() as u16)?;

        let serialize_string = |f: &mut File, s: &str| -> std::io::Result<()> {
            if s.len() > std::u16::MAX as usize {
                panic!("String too large to serialize.");
            }
            let s = s.as_bytes();
            f.write_u16::<BigEndian>(s.len() as u16)?;
            f.write_all(s)?;
            Ok(())
        };

        for b in BLOCK_IDS.iter() {
            serialize_string(&mut file, b)?;
        }

        let mut encoder = BufWriter::new(ZlibEncoder::new(file, Compression::Best));

        // Generate reverse lookup table for block ids.
        let mut block_lookup_table: std::collections::HashMap<&'static str, usize> =
            std::collections::HashMap::new();
        for (idx, val) in BLOCK_IDS.iter().enumerate() {
            block_lookup_table.insert(val, idx);
        }

        println!(" [+] Writing node types");
        // Write node types.
        for x in 0..d.0 {
            for z in 0..2 {
                for y in 0..d.1 {
                    let t = if z == 0 {
                        "stone"
                    } else {
                        self.get(x, y).minetest_type()
                    };

                    encoder.write_u16::<BigEndian>(
                        *block_lookup_table
                            .get(t)
                            .unwrap_or_else(|| panic!("Block type {:?} not found?", t))
                            as u16,
                    )?;
                }
            }
        }

        println!(" [+] Writing param1");
        // Write param1
        for _ in 0..2 {
            for _ in 0..d.0 {
                for _ in 0..d.1 {
                    encoder.write_u8(0)?;
                }
            }
        }

        println!(" [+] Writing param2");
        // Write param2
        for x in 0..d.0 {
            for z in 0..2 {
                for y in 0..d.1 {
                    encoder.write_u8(if z == 0 {
                        0
                    } else {
                        self.get(x, y).get_param2()
                    })?;
                }
            }
        }

        Ok(())
    }
}
