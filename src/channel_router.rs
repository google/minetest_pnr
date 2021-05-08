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

use std::cmp;
use std::collections::HashMap;

struct Ranges {
    ranges: Vec<std::ops::Range<usize>>,
}

impl Ranges {
    fn new() -> Self {
        Ranges { ranges: Vec::new() }
    }

    fn add(&mut self, start: usize, end: usize) {
        let (start, end) = (cmp::min(start, end), cmp::max(start, end) + 1);
        self.ranges.push(std::ops::Range { start, end });
    }

    fn contains(&self, start: usize, end: usize) -> bool {
        let (start, end) = (cmp::min(start, end), cmp::max(start, end));
        (start..=end).any(|v| self.ranges.iter().any(|r| r.contains(&v)))
    }

    fn contains_range(&self, range: &std::ops::Range<usize>) -> bool {
        self.contains(range.start, range.end)
    }

    fn range_sum(&self) -> usize {
        self.ranges.iter().map(|r| r.end - r.start).sum()
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ChannelState {
    Free,
    // Occupied means no connection. This is the same as a constant false.
    Occupied,
    // Constant true.
    Constant,
    Net(usize),
}
pub type ChannelLayout = [ChannelState];

impl ChannelState {
    pub fn is_free(&self) -> bool {
        self == &ChannelState::Free
    }

    pub fn contains_net(&self) -> bool {
        matches!(self, ChannelState::Net(_))
    }

    pub fn is_constant_on(&self) -> bool {
        matches!(self, ChannelState::Constant)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ChannelOp {
    Move,
    Copy,
}

#[derive(Debug, Clone)]
pub struct WireConnection {
    pub from: usize,
    pub to: Vec<usize>,
    pub mode: ChannelOp,
}

#[derive(Debug)]
pub struct ChannelSubState {
    pub wires: Vec<WireConnection>,
    pub occupancy_map: bitmap::Bitmap<Vec<usize>, bitmap::OneBit>,
}

#[derive(Debug)]
struct Task {
    net: usize,
    from: usize,
    to: Vec<usize>,
}

impl Task {
    fn channel_range_required(&self) -> std::ops::Range<usize> {
        let from = [self.from];
        let min = self.to.iter().chain(&from).min().unwrap();
        let max = self.to.iter().chain(&from).max().unwrap();
        std::ops::Range {
            start: *min,
            end: max + 1,
        }
    }

    fn channel_width_required(&self) -> usize {
        let r = self.channel_range_required();
        r.end - r.start
    }

    fn occupied_target_pins(&self, layout: &ChannelLayout) -> Vec<usize> {
        let mut occupied = Vec::new();
        for &idx in &self.to {
            if layout[idx].contains_net() && layout[idx] != ChannelState::Net(self.net) {
                occupied.push(idx);
            }
        }
        occupied
    }

    // Returns how 'good' a new 'from' position is for this task (when evicting)
    // so that we can prefer nice spots.
    fn eviction_cost(&self, new_pos: usize) -> usize {
        let min = self.to.iter().min().unwrap();
        let max = self.to.iter().max().unwrap();

        let dist = (self.from as isize - new_pos as isize).abs() as usize;

        if new_pos > *max {
            2 * (new_pos - *max) + dist
        } else if new_pos < *min {
            2 * (*min - new_pos) + dist
        } else {
            dist
        }
    }
}

#[derive(Default)]
struct RouteTasks {
    // source idx -> vec<target idx>
    tasks: HashMap<usize, Vec<usize>>,
}

impl RouteTasks {
    fn add(&mut self, from: usize, to: usize) {
        if let Some(k) = self.tasks.get_mut(&from) {
            k.push(to);
        } else {
            self.tasks.insert(from, vec![to]);
        }
    }

    fn into_tasks(mut self, src: &ChannelLayout) -> Vec<Task> {
        self.tasks
            .drain()
            .map(|(k, v)| {
                let net = match src[k] {
                    ChannelState::Net(i) => i,
                    _ => unreachable!(),
                };
                Task {
                    net,
                    from: k,
                    to: v,
                }
            })
            .collect::<Vec<_>>()
    }
}

pub fn route_channel(start: &ChannelLayout, end: &ChannelLayout) -> Vec<ChannelSubState> {
    let mut state = start.to_owned();
    // Expand the state to be at least end.len() wide.
    while state.len() < end.len() {
        state.push(ChannelState::Free);
    }

    let mut tasks = RouteTasks::default();

    for end_idx in 0..end.len() {
        if !end[end_idx].contains_net() || end[end_idx] == state[end_idx] {
            continue;
        }

        let state_idx = state
            .iter()
            .position(|v| v == &end[end_idx])
            .unwrap_or_else(|| panic!("Required field '{:?}' not found", end[end_idx]));
        tasks.add(state_idx, end_idx);
    }

    let mut tasks = tasks.into_tasks(&state);
    // Order by how much of the channel this task occupies.
    tasks.sort_by_key(|k| k.channel_width_required());

    let mut steps: Vec<ChannelSubState> = Vec::new();

    loop {
        // Ranges of the channel that is currently occupied.
        let mut ranges = Ranges::new();
        // Instruction on how to connect pins in the current part of the channel.
        let mut wires = Vec::new();
        // To detect if we were unable to do anything due to blocked pins.
        let old_task_len = tasks.len();

        tasks = tasks
            .drain(0..tasks.len())
            .filter(|task| {
                // Speed things up by only 'enforcing' 50% channel utilization.
                if ranges.range_sum() > (cmp::max(state.len(), end.len()) / 2) {
                    return true;
                }

                // Do we have the required part of the channel available?
                if ranges.contains_range(&task.channel_range_required()) {
                    return true;
                }

                let blocking_pins = task.occupied_target_pins(&state);
                if blocking_pins.is_empty() {
                    // Targets are free, directly move (or copy) it there.

                    let keep = if task.from >= end.len() || state[task.from] != end[task.from] {
                        state[task.from] = ChannelState::Free;
                        false
                    } else {
                        true
                    };

                    wires.push(WireConnection {
                        from: task.from,
                        to: task.to.clone(),
                        mode: if keep {
                            ChannelOp::Copy
                        } else {
                            ChannelOp::Move
                        },
                    });

                    let r = task.channel_range_required();
                    // -1 here since .add() + channel_range_required() will do +1.
                    ranges.add(r.start, r.end - 1);

                    for &to in &task.to {
                        state[to] = ChannelState::Net(task.net);
                    }

                    // We successfully handled this one.
                    return false;
                }

                true
            })
            .collect::<Vec<_>>();

        // We were unable to handle any tasks -> we need to evict some channels.
        if old_task_len == tasks.len() {
            // Find available positions where we can evict to.
            let mut free_positions = state
                .iter()
                .enumerate()
                .filter(|(_, v)| !v.contains_net())
                .map(|(k, _)| k)
                .filter(|&k| k >= end.len() || !end[k].contains_net())
                .collect::<Vec<_>>();

            if free_positions.is_empty() {
                println!("[!] No free positions found, expanding channel");
                // Make sure that we have some room, scaling with the number of
                // remaining tasks as a random tradeoff.
                for _ in 0..(tasks.len() / 10 + 1) {
                    state.push(ChannelState::Free);
                    free_positions.push(state.len() - 1);
                }
            }

            for task_idx in 0..tasks.len() {
                let blocking_pins = tasks[task_idx].occupied_target_pins(&state);
                for to_evict in blocking_pins {
                    // Find corresponding task.
                    let task_idx_to_evict = tasks
                        .iter()
                        .position(|t| t.from == to_evict)
                        .unwrap_or_else(|| panic!("Could not find task blocking {}", to_evict));

                    // Find a good place for this task to evict to.
                    free_positions.sort_by(|&a, &b| {
                        // Comparing in the opposite order on purpose here so
                        // that we can use pop() later.
                        tasks[task_idx_to_evict]
                            .eviction_cost(b)
                            .cmp(&tasks[task_idx_to_evict].eviction_cost(a))
                    });

                    let from = tasks[task_idx_to_evict].from;
                    let new_pos = *free_positions.last().unwrap();

                    // Check whether the space is actually available.
                    let req_range = std::ops::Range {
                        start: cmp::min(from, new_pos),
                        end: cmp::max(from, new_pos) + 1,
                    };

                    if !ranges.contains_range(&req_range) {
                        free_positions.pop();
                        ranges.add(from, new_pos);
                        wires.push(WireConnection {
                            from,
                            to: vec![new_pos],
                            mode: ChannelOp::Move,
                        });
                        tasks[task_idx_to_evict].from = new_pos;
                        state[new_pos] = ChannelState::Net(tasks[task_idx_to_evict].net);
                        state[to_evict] = ChannelState::Free;
                    }
                }
            }
        }

        let mut bitmap =
            bitmap::Bitmap::from_storage(state.len(), (), vec![0; (state.len() + 63) / 64])
                .unwrap();
        for idx in state
            .iter()
            .enumerate()
            .filter(|(_, v)| v.contains_net())
            .map(|(k, _)| k)
        {
            bitmap.set(idx, 1);
        }

        steps.push(ChannelSubState {
            wires,
            occupancy_map: bitmap,
        });
        if tasks.is_empty() {
            return steps;
        }
    }
}
