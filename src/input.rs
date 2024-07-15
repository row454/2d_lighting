use std::collections::HashMap;
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey::{self, Code}},
};
pub struct InputHandler {
    key_map: HashMap<KeyCode, Vec<usize>>,
    key_state: Vec<KeyState>,
    control_map: HashMap<Control, Vec<usize>>,
    pair_count: usize,
}

// each key_state entry will represents a unique pairing of a key and its control. for each key, there is many states, and for each control, there is many states.

impl InputHandler {
    pub fn new() -> InputHandler {
        InputHandler {
            key_map: HashMap::new(),
            key_state: Vec::new(),
            control_map: HashMap::new(),
            pair_count: 0,
        }
    }
    pub fn register_control(&mut self, key: KeyCode, control: Control) {
        match (
            self.key_map.get_mut(&key),
            self.control_map.get_mut(&control),
        ) {
            (None, None) => {
                self.key_map.insert(key, vec![self.pair_count]);
                self.control_map.insert(control, vec![self.pair_count]);
            }
            (None, Some(vec)) => {
                self.key_map.insert(key, vec![self.pair_count]);
                vec.push(self.pair_count);
            }
            (Some(vec), None) => {
                vec.push(self.pair_count);
                self.control_map.insert(control, vec![self.pair_count]);
            }
            (Some(keys), Some(controls)) => {
                for index in keys.iter() {
                    if controls.contains(index) {
                        return;
                    }
                }
                keys.push(self.pair_count);
                controls.push(self.pair_count);
            }
        }
        self.key_state.push(KeyState::default());
        self.pair_count += 1;
    }
    pub fn handle_input(&mut self, physical_key: PhysicalKey, state: ElementState) {
        if let Code(key) = physical_key {
            if let Some(indices) = self.key_map.get(&key) {
                let state = match state {
                    winit::event::ElementState::Pressed => KeyState::Pressed,
                    winit::event::ElementState::Released => KeyState::Released,
                };
                for &index in indices {
                    self.key_state[index] = state;
                }
            }
        }
    }
    pub fn update(&mut self) {
        for state in self.key_state.iter_mut() {
            match state {
                KeyState::Pressed => *state = KeyState::Held,
                KeyState::Released => *state = KeyState::Released,
                _ => (),
            }
        }
    }
    pub fn is_pressed(&self, control: Control) -> bool {
        if let Some(indices) = self.control_map.get(&control) {
            let mut result = false;
            for &index in indices {
                result = result || self.key_state[index].is_pressed();
            }
            result
        } else {
            false
        }
    }
    pub fn reset_states(&mut self) {
        for state in self.key_state.iter_mut() {
            *state = KeyState::default();
        }
    }
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum Control {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
}
#[derive(Copy, Clone, Debug)]
enum KeyState {
    Pressed,
    Held,
    Released,
    Inactive,
}
impl KeyState {
    pub fn is_pressed(&self) -> bool {
        match self {
            Self::Pressed | Self::Held => true,
            Self::Released | Self::Inactive => false,
        }
    }
}
impl Default for KeyState {
    fn default() -> Self {
        Self::Inactive
    }
}
