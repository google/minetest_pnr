# MinetestPnR
Simple 2D placer & router for [minetest] developed as part of a [Google CTF] challenge ([video writeup](https://www.youtube.com/watch?v=nI8Q1bqT8QU)).
This also means that the project is mostly aimed towards creating this challenge,
not towards being feature complete and being well engineered, so don't expect
optimal/minimal output.

It currently supports only basic gates (i.e. **no DFF / latches**), so I'm afraid you
can't just place your RISC-V CPU in minetest, feel free to send pull requests though ;).

Supported gates:
 - AND
 - NAND
 - ANDNOT
 - OR
 - NOR
 - ORNOT
 - NOT
 - XNOR
 - XOR

The result will look similar to this (text version):
```Text
 ░──────────────────¬─────────────┐┌───┐  ┌───────┐  ┌─────┐        ┌──┐  ┌─┐
 ░────────┬─────────¬─────────┐   ││   ^──┘       ^──┘     ^─────»──┘  ^──┘ ^────▓
 ░────────╂──────┬──¬─────────╂───╂┘┌──┘  ┌───────┘  ┌─────┘        ┌──┘  ┌─┘
 ░────────╂──────╂┬─¬─────┬───╂───╂─╂──┐  │┌──────┐  │┌────┐        │┌─┐  │
 ░──────┬─╂──────╂╂─¬─────╂───╂───╂─╂┐ ^──┘│      ^──┘│    v─────¬──┘│ ^──┘
 ░──────╂─╂─┬────╂╂─»──┐  │   │   │ │├─┘   │┌─────┘  ┌╂────┘        ┌╂─┘
 ░──────╂─╂─╂┬───╂╂─¬──╂──╂───╂──┐└─╂╂─┐  ┌╂╂─────┐  ││┌───┐        ││
 ░──────╂┐└─╂╂───╂╂─»──╂──╂───╂──╂──┤│ ^──╂┘│     v──╂┘│   ^─────»──╂┘
 ░──────╂╂──╂╂───╂╂─»──╂──╂───╂──╂──╂╂─┘  │ │┌────┘  │ │┌──┘  ┌──┐  │
 ░──────╂╂──╂╂──┬╂╂─¬──╂──╂───╂┐┌╂──╂╂─┐  │┌╂╂────┐  │┌╂╂──┐  │  ^──┘
 ░──────╂╂──╂╂─┐└╂╂─»──╂┐ │   ││││  ││ v──┘│││    v──┘│││  ^──┘┌─┘
 ░────┬─╂╂──╂╂─╂─╂╂─¬──╂╂─╂───╂╂╂╂┐┌╂╂─┘   │││┌───┘  ┌╂╂╂──┘   │
 ░────╂─╂╂──╂╂┬╂─╂╂─¬──╂╂┐│   ││││││││     ││││┌──┐  ││││┌─┐   │
 ░────╂─╂╂┐ ├╂╂╂─╂╂─┐  ││││   ││││││││     │││││  ^──╂╂┘││ ^───┘
 ░────╂─╂╂╂┐││││ ││ v──╂╂╂╂───╂╂╂╂╂╂╂╂─»───╂┘│││┌─┘  ││┌╂╂─┘
 ░────╂┐││├╂╂╂╂╂─╂╂─┘  └╂╂╂───╂╂╂╂╂╂╂╂─┐  ┌╂─╂╂╂╂─┐  │││││
 ░───┐└╂╂╂╂╂╂╂╂╂─╂╂─»───╂╂╂─┐ ││││││││ v──╂┘ ││││ ^──╂┘│││
 ░───╂─╂╂╂╂╂╂╂╂╂─╂╂─»───╂╂╂─╂─╂╂┘└╂╂╂╂─┘  │ ┌╂╂╂╂─┘  │ │││
 ░──┐│ │└╂╂╂╂╂╂╂─╂╂─»───╂╂╂─╂┐└╂──╂╂╂╂─┐  │ │││││    │ │││
 ░──╂╂─╂─╂╂╂╂╂╂╂─╂╂─¬───╂╂╂─╂╂─╂──╂┘││ v──╂─╂╂╂╂╂─»──╂─╂┘│
    │└─╂─╂╂╂╂╂╂╂─╂╂─┐   └╂╂─╂╂─╂──╂─╂╂─┘  │┌╂╂╂╂╂─┐  │ │ │
    │  │ │││││││ ││ v────╂╂─╂╂─╂──╂─╂╂─»──╂╂╂┘│││ ^──┘ │ │
    └──╂─╂╂╂╂╂╂╂─╂╂─┘    ││ ││ │  │ ││    │││┌╂╂╂─┘    │ │
       │ └╂╂╂╂╂╂─╂╂─┐    ││ ││ │  │ ││    │││││││      │ │
       │  ││││││ ││ v────╂╂─╂╂─╂──╂─╂╂─»──╂╂╂╂┘││      │ │
       └──╂╂╂╂╂╂─╂╂─┘    ││ ││ │  │ ││    ││││ ││      │ │
          ││││││ ├╂─┐    ││ ││ │  │ ││    ││││ ││      │ │
          ││││││ ││ ^────╂╂─╂╂─╂──╂─╂╂─¬──┘│││ ││      │ │
          │││││├─╂╂─┘    │└─╂╂─╂──╂─╂╂─┐   │││ ││      │ │
          ││└╂╂╂─╂╂─┐    │  ││ │  │ ││ v───╂╂╂─┘│      │ │
          ││ │││ ││ ^────╂─┐└╂─╂──╂─╂╂─┘   │││  │      │ │
          └╂─╂╂╂─╂╂─┘    │ │ │ │  │ └╂─┐   │││  │      │ │
           │ ├╂╂─╂╂─┐    │ │ │ │  │  │ v───╂╂╂──┘      │ │
           │ │││ ││ ^────╂┐│ │ └──╂──╂─┘   │││         │ │
           ├─╂╂╂─╂╂─┘    └╂╂─╂────╂──╂─┐   │││         │ │
           │ └╂╂─╂╂─┐     ││ │    │  │ v───╂┘│         │ │
           │  ││ ││ v────┐││ └────╂──╂─┘   │ │         │ │
           └──╂╂─╂╂─┘    │││      │  └─┐   │ │         │ │
              ││ └╂─┐    │││      │    v───┘ │         │ │
              ││  │ v──┐┌╂╂╂──────╂────┘     │         │ │
              │└──╂─┘  │││││      └────┐     │         │ │
              └───╂─»──╂┘│││           v─────┘         │ │
                  └─»──╂─╂╂╂───────────┘               │ │
                       │ ││└───────────┐               │ │
                       │ ││            v──────────¬────╂─┘
                       │ │└────────────┘               │
                       │ └─────────────┐               │
                       │               ^──────────»────┘
                       └───────────────┘
```

For ingame footage, look at the CTF writeup video mentioned above.

## What does it do?
MinetestPnR takes a synthesized circuit, places the components and routes them (= wires things together).
This means that it'll allow you to write your circuit in a HDL (e.g. `verilog`), synthesize it (e.g. using `yosys`)
and then use it inside your minetest world.

## How can I use it?
Putting your circuit in minetest takes three steps:
 - Generate the minetest schematic file
 - Place it in minetest
 - Fix the mesecon parts. More on this below.

### Generating minetest schematic
- Synthesize your circuit and create a json file containing the basic blocks using [yosys].
  Example command: `yosys -p 'synth; abc -g AND,OR,XOR,XNOR,ANDNOT,ORNOT; write_json schematic.json' schematic.v`
- Place & route the resulting `schematic.json` file, creating a MTS(minetest schematic) file using this project:
  `cargo run --release -- ./schematic.json --write_mts schematic.mts`

### Placing MTS in minetest using worldedit
- Install [mesecons] + [worldedit]
- Create world in minetest (`type=single node` if you only want to have the circuit in the world)
- Remember to enable both mods
- Create `schems` directory in the world directory (`worlds/${worldname}/schems`)
- Generate MTS file using the project, place it in that folder
- Start minetest + load your map
- Set pos1 (`//1` / `//fixedpos set1 0 1 0`) where the schematic should be placed
- `//mtschemplace <name without mts>` to place the schematic

### Fixing mesecon wires / gates
- Fix mesecon wires + gates - those blocks introduce some internal state that is not created when
  placed using worldedit. This can be done by adding a function to the mesecons code that will
  set up that internal state. This can be done by adding this function:

```lua
local function fix_single_chunk(pos)
    local found_nodes = minetest.find_nodes_in_area(pos, vector.add(pos, { x = 16, y = 16, z = 16 }), {
        "mesecons_gates:diode_off",
        "mesecons_gates:not_off",
        "mesecons_gates:and_off",
        "mesecons_gates:nand_off",
        "mesecons_gates:xor_off",
        "mesecons_gates:nor_off",
        "mesecons_gates:or_off",
        "mesecons:mesecons_off",
        "mesecons:wire_00000000_off",
    })
    local cnt = 0

    for i=0, #found_nodes do
        if (found_nodes[i] ~= nil) then
            mesecon.on_placenode(found_nodes[i], minetest.get_node(found_nodes[i]))
            cnt = cnt + 1
        end
    end

    return cnt
end

-- Adapted from:
-- https://rubenwardy.com/minetest_modding_book/en/map/environment.html
local function emerge_callback(pos, action,
        num_calls_remaining, context)
    -- On first call, record number of blocks
    if not context.total_blocks then
        context.total_blocks  = num_calls_remaining + 1
        context.loaded_blocks = 0
        context.nodes_fixed = 0
    end

    context.loaded_blocks = context.loaded_blocks + 1
    local perc = 100 * context.loaded_blocks / context.total_blocks
    local msg  = string.format("Handling block %d/%d (%.2f%%)",
            context.loaded_blocks, context.total_blocks, perc)
    context.nodes_fixed = context.nodes_fixed + fix_single_chunk(pos)
    minetest.chat_send_all(msg)

    -- Are we done yet?
    if context.total_blocks == context.loaded_blocks then
        minetest.chat_send_all("Done, " .. context.nodes_fixed .. " nodes fixed")
    end
end

minetest.register_chatcommand("fix_gates", {
    params = "",
    description = "Fix gates by triggering I/O reevaluation",
    func = function(name, param)
        local context = {}
        minetest.emerge_area({x=0, y=2, z=0}, {x=2000, y=2, z=2000}, emerge_callback, context)
        return true, "Emerge started"
    end,
})
```

e.g. at the end of `mesecons_gates/init.lua`.
Then load up the map and send `/fix_gates` in chat.
This sometimes does not seem to work reliably, so you might need to use it multiple
times when standing at different locations to cover different chunks.

**Note that the snippet contains the area where the wires should be fixed** (`minetest.emerge_area({x=0, y=2, z=0}, {x=2000, y=2, z=2000}`)
so you might want to adjust this.

## Caveats
### My circuit is too large!
The application uses a 8k * 8k tiles canvas where the circuit gets placed onto.
The maximum size that the MTS format supports is 64k * 64k, so depending on how
big your circuit is you can place your bigger circuit by changing the canvas
size in `src/canvas.rs`:
```rust
const CANVAS_MAX_W: usize = 8 * 1024;
const CANVAS_MAX_H: usize = 8 * 1024;
```

Why not using 64k * 64k by default? 64k * 64k = 4G - that's a lot of ram ;).
**NOTE**: The minetest source code is not consistent whether the dimensions are
stored as `u16` or `i16`, so the code might need additional patches in
`serialize_to_mts` if your circuit is bigger than 32k * 32k.

## Disclaimer
This is not an officially supported Google product.

[minetest]: http://www.minetest.net/
[worldedit]: https://github.com/Uberi/Minetest-WorldEdit
[mesecons]: http://mesecons.net/
[yosys]: http://www.clifford.at/yosys/
[Google CTF]: https://capturetheflag.withgoogle.com
