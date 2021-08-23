use minifb::{Key, Window, WindowOptions};
use rodio::{source::SineWave, OutputStream, Sink};
use std::time::{Duration, Instant};

mod chip8;

const WINDOW_WIDTH: usize = 640;
const WINDOW_HEIGHT: usize = 320;
const FRAMES_PER_SEC: f64 = 60.;
const CYCLES_PER_SEC: f64 = 600.;

fn main() {
    let rom_location = std::env::args().nth(1).expect("Must specify rom location");
    let mut cpu = create_cpu(&rom_location);
    let (_stream, sink) = create_audio();
    let mut window = create_window();

    let mut last_cycle_time = Instant::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        update_keys(&window, &mut cpu);
        last_cycle_time = update_cpu(last_cycle_time.elapsed(), &mut cpu);
        update_audio(&cpu, &sink);
        update_window(&cpu, &mut window);
    }
}

fn create_cpu(rom_location: &str) -> chip8::Cpu {
    let rom = std::fs::File::open(&rom_location).expect("Failed to open rom");
    chip8::Cpu::new(rom)
}

fn update_cpu(time_since_last_process: Duration, cpu: &mut chip8::Cpu) -> Instant {
    let cycle_count = (CYCLES_PER_SEC * time_since_last_process.as_secs_f64()).round() as u64;
    let start_time = Instant::now();
    for _ in 0..cycle_count {
        cpu.cycle();
    }
    start_time
}

fn update_keys(window: &Window, cpu: &mut chip8::Cpu) {
    if let Some(keys) = window.get_keys() {
        let key_values = keys
            .into_iter()
            .filter_map(|key| match key {
                Key::Key1 => Some(1),
                Key::Key2 => Some(2),
                Key::Key3 => Some(3),
                Key::Key4 => Some(0xC),

                Key::Q => Some(4),
                Key::W => Some(5),
                Key::E => Some(6),
                Key::R => Some(0xD),

                Key::A => Some(7),
                Key::S => Some(8),
                Key::D => Some(9),
                Key::F => Some(0xE),

                Key::Z => Some(0xA),
                Key::X => Some(0),
                Key::C => Some(0xB),
                Key::V => Some(0xF),

                _ => None,
            })
            .collect();
        cpu.set_keys(key_values);
    }
}

fn create_audio() -> (OutputStream, Sink) {
    let (stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();
    let source = SineWave::new(512);
    sink.pause();
    sink.append(source);
    (stream, sink)
}

fn update_audio(cpu: &chip8::Cpu, sink: &Sink) {
    match cpu.beep() {
        true => sink.play(),
        false => sink.pause(),
    }
}

fn create_window() -> Window {
    let mut window = Window::new(
        "Chip-8",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions::default(),
    )
    .unwrap();
    window.limit_update_rate(Some(Duration::from_secs_f64(1. / FRAMES_PER_SEC)));
    window
}

fn update_window(cpu: &chip8::Cpu, window: &mut Window) {
    let buffer = cpu
        .display()
        .iter()
        .map(|x| match x {
            true => 255,
            false => 0,
        })
        .collect::<Vec<_>>();
    window
        .update_with_buffer(&buffer, chip8::DISPLAY_WIDTH, chip8::DISPLAY_HEIGHT)
        .unwrap();
}
