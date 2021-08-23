use std::io::Read;
use std::time::{Duration, Instant};

type Opcode = u16;

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;

const FONT_START: usize = 0x050;
const FONT_HEIGHT: usize = 5;
const KEY_COUNT: usize = 16;
const MEMORY_SIZE: usize = 4096;
const PROGRAM_COUNTER_START: u16 = 0x200;
const TIMER_TICKS_PER_SEC: f64 = 60.;
const V_COUNT: usize = 16;
const V_CARRY_FLAG: usize = 15;

const FONTS: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Cpu {
    memory: [u8; MEMORY_SIZE],
    program_counter: u16,

    i: u16,
    v: [u8; V_COUNT],

    stack: Vec<u16>,

    delay_timer: u8,
    sound_timer: u8,
    prev_timer_time: Instant,

    display: [bool; DISPLAY_WIDTH * DISPLAY_HEIGHT],
    display_modified: bool,

    pressed_keys: [bool; KEY_COUNT],
}

impl Cpu {
    pub fn new(mut rom: std::fs::File) -> Self {
        let mut cpu = Self {
            memory: [0; MEMORY_SIZE],
            program_counter: PROGRAM_COUNTER_START,

            i: 0,
            v: [0; V_COUNT],

            stack: Vec::with_capacity(16),

            delay_timer: 0,
            sound_timer: 0,
            prev_timer_time: Instant::now(),

            display: [false; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            display_modified: false,

            pressed_keys: [false; KEY_COUNT],
        };
        (&FONTS[..])
            .read_exact(&mut cpu.memory[FONT_START..(FONT_START + FONTS.len())])
            .unwrap();
        rom.read_exact(
            &mut cpu.memory[PROGRAM_COUNTER_START as usize
                ..(PROGRAM_COUNTER_START as usize + rom.metadata().unwrap().len() as usize)],
        )
        .unwrap();
        cpu
    }

    pub fn display(&self) -> [bool; DISPLAY_WIDTH * DISPLAY_HEIGHT] {
        self.display
    }

    pub fn beep(&self) -> bool {
        self.sound_timer > 0
    }

    pub fn set_keys(&mut self, keys: Vec<usize>) {
        self.pressed_keys = [false; KEY_COUNT];
        for key in keys {
            if key < KEY_COUNT {
                self.pressed_keys[key] = true;
            }
        }
    }

    pub fn cycle(&mut self) {
        if self.prev_timer_time.elapsed() >= Duration::from_secs_f64(1. / TIMER_TICKS_PER_SEC) {
            self.prev_timer_time = Instant::now();
            self.process_timers();
        }

        let opcode = self.fetch();
        self.process_opcode(opcode);
    }

    fn process_timers(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    fn fetch(&mut self) -> Opcode {
        let opcode = ((self.memory[self.program_counter as usize] as u16) << 8)
            | (self.memory[self.program_counter as usize + 1] as u16);
        self.program_counter += 2;
        opcode
    }

    fn process_opcode(&mut self, opcode: Opcode) {
        let op_1 = (opcode & 0xF000) >> 12;
        let op_2 = (opcode & 0x0F00) >> 8;
        let op_3 = (opcode & 0x00F0) >> 4;
        let op_4 = opcode & 0x000F;

        let x = ((opcode & 0x0F00) >> 8) as usize;
        let vx = self.v[x];
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let vy = self.v[y];
        let nnn = opcode & 0x0FFF;
        let nn = (opcode & 0x00FF) as u8;
        let n = (opcode & 0x000F) as u8;

        match (op_1, op_2, op_3, op_4) {
            (0, 0, 0xE, 0) => {
                self.display = [false; DISPLAY_WIDTH * DISPLAY_HEIGHT];
                self.display_modified = true;
            }
            (0, 0, 0xE, 0xE) => self.program_counter = self.stack.pop().unwrap(),
            (1, _, _, _) => self.program_counter = nnn,
            (2, _, _, _) => {
                self.stack.push(self.program_counter);
                self.program_counter = nnn;
            }
            (3, _, _, _) => {
                if vx == nn {
                    self.program_counter += 2;
                }
            }
            (4, _, _, _) => {
                if vx != nn {
                    self.program_counter += 2;
                }
            }
            (5, _, _, 0) => {
                if vx == vy {
                    self.program_counter += 2;
                }
            }
            (6, _, _, _) => self.v[x] = nn,
            (7, _, _, _) => self.v[x] = vx.wrapping_add(nn),
            (8, _, _, 0) => self.v[x] = vy,
            (8, _, _, 1) => self.v[x] = vx | vy,
            (8, _, _, 2) => self.v[x] = vx & vy,
            (8, _, _, 3) => self.v[x] = vx ^ vy,
            (8, _, _, 4) => {
                let (sum, overflow) = vx.overflowing_add(vy);
                self.v[x] = sum;
                self.v[V_CARRY_FLAG] = overflow as u8;
            }
            (8, _, _, 5) => {
                let (sub, overflow) = vx.overflowing_sub(vy);
                self.v[x] = sub;
                self.v[V_CARRY_FLAG] = !overflow as u8;
            }
            (8, _, _, 6) => {
                self.v[x] = vx >> 1;
                self.v[V_CARRY_FLAG] = vx & 1;
            }
            (8, _, _, 7) => {
                let (sub, overflow) = vy.overflowing_sub(vx);
                self.v[x] = sub;
                self.v[V_CARRY_FLAG] = !overflow as u8;
            }
            (8, _, _, 0xE) => {
                self.v[x] = vx << 1;
                self.v[V_CARRY_FLAG] = if (vx & 0x80) == 0 { 0 } else { 1 };
            }
            (9, _, _, 0) => {
                if vx != vy {
                    self.program_counter += 2;
                }
            }
            (0xA, _, _, _) => self.i = nnn,
            (0xB, _, _, _) => self.program_counter = nnn + self.v[0] as u16,
            (0xC, _, _, _) => self.v[x] = rand::random::<u8>() & nn,
            (0xD, _, _, _) => self.display_opcode(vx, vy, n),
            (0xE, _, 9, 0xE) => {
                if self.pressed_keys[vx as usize] {
                    self.program_counter += 2;
                }
            }
            (0xE, _, 0xA, 1) => {
                if !self.pressed_keys[vx as usize] {
                    self.program_counter += 2;
                }
            }
            (0xF, _, 0, 7) => self.v[x] = self.delay_timer,
            (0xF, _, 0, 0xA) => match self.pressed_keys.iter().position(|x| x == &true) {
                Some(index) => self.v[x] = index as u8,
                None => self.program_counter -= 2,
            },
            (0xF, _, 1, 5) => self.delay_timer = self.v[x],
            (0xF, _, 1, 8) => self.sound_timer = self.v[x],
            (0xF, _, 1, 0xE) => self.i = self.i.wrapping_add(vx as u16),
            (0xF, _, 2, 9) => {
                self.i = (FONT_START + (FONT_HEIGHT * (vx & 0x0F) as usize)) as u16;
            }
            (0xF, _, 3, 3) => {
                self.memory[self.i as usize] = vx / 100 % 10;
                self.memory[(self.i + 1) as usize] = vx / 10 % 10;
                self.memory[(self.i + 2) as usize] = vx % 10;
            }
            (0xF, _, 5, 5) => {
                for index in 0..=x {
                    self.memory[self.i as usize + index] = self.v[index];
                }
            }
            (0xF, _, 6, 5) => {
                for index in 0..=x {
                    self.v[index] = self.memory[self.i as usize + index];
                }
            }

            _ => println!("unsupported opcode 0x{:04X}", opcode),
        };
    }

    fn display_opcode(&mut self, x: u8, y: u8, height: u8) {
        let x = x as usize % DISPLAY_WIDTH;
        let y = y as usize % DISPLAY_HEIGHT;
        let height = height as usize;

        self.v[V_CARRY_FLAG] = 0;
        for row in 0..height {
            let sprite = self.memory[self.i as usize + row];
            for col in 0..8 {
                let bit = (sprite >> (7 - col)) & 1;
                if (bit == 1) && (x + col < DISPLAY_WIDTH) && (y + row < DISPLAY_HEIGHT) {
                    let pixel = &mut self.display[(x + col) + (DISPLAY_WIDTH * (y + row))];
                    if *pixel {
                        self.v[V_CARRY_FLAG] = 1;
                    }
                    *pixel = !*pixel;
                }
            }
        }

        self.display_modified = true;
    }
}
