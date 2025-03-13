use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc};
use std::thread;
use std::time::Duration;
use enigo::{Enigo, Key, KeyboardControllable, MouseButton, MouseControllable};
use rdev::{listen, Event, EventType};

#[derive(Debug, Clone)]
struct Config {
    start_key: String,
    stop_key: String,
    click_key: String,
    cps: u32,
}

impl Config {
    fn load() -> Option<Config> {
        let content = fs::read_to_string("config.ini").ok()?;
        let mut start_key = String::new();
        let mut stop_key = String::new();
        let mut click_key = String::new();
        let mut cps = 10;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("start_key=") {
                start_key = line.replace("start_key=", "").trim().to_string();
            } else if line.starts_with("stop_key=") {
                stop_key = line.replace("stop_key=", "").trim().to_string();
            } else if line.starts_with("click_key=") {
                click_key = line.replace("click_key=", "").trim().to_string();
            } else if line.starts_with("cps=") {
                cps = line.replace("cps=", "").trim().parse::<u32>().unwrap_or(10);
            }
        }
        Some(Config { start_key, stop_key, click_key, cps })
    }
    
    fn save(&self) -> std::io::Result<()> {
        let content = format!("start_key={}\nstop_key={}\nclick_key={}\ncps={}\n", self.start_key, self.stop_key, self.click_key, self.cps);
        fs::write("config.ini", content)
    }
}

fn capture_input(prompt: &str) -> String {
    println!("{}", prompt);
    println!("Press any button...");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let callback = move |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) => { let key_str = format!("{:?}", key); let _ = tx.send(key_str); },
                EventType::ButtonPress(button) => { let button_str = format!("{:?}", button); let _ = tx.send(button_str); },
                _ => {}
            }
        };
        if let Err(e) = listen(callback) { eprintln!("Error listening to events: {:?}", e); }
    });
    match rx.recv() {
        Ok(val) => { println!("Captured: {}", val); val },
        Err(e) => { eprintln!("Error receiving input: {:?}", e); String::new() }
    }
}

fn create_config() -> Config {
    let start_key = capture_input("Press the key for starting the autoclicker:");
    let stop_key = capture_input("Press the key for stopping the autoclicker:");
    let click_key = capture_input("Press the key/button to be clicked by the autoclicker:");
    println!("Enter the number of clicks per second (number):");
    let mut cps_str = String::new();
    io::stdin().read_line(&mut cps_str).expect("Error reading input");
    let digits: String = cps_str.trim().chars().filter(|c| c.is_digit(10)).collect();
    let cps = if let Ok(value) = digits.parse::<u32>() { if value > 0 { value } else { println!("Entered zero value, defaulting to 10"); 10 } } else { println!("Invalid input, defaulting to 10"); 10 };
    Config { start_key, stop_key, click_key, cps }
}

fn main() {
    println!("▄▀█ █░█ █▀▀ █░░ █ █▀▀ █▄▀   ▄█ ░ █▀█");
    println!("█▀█ █▄█ █▄▄ █▄▄ █ █▄▄ █░█   ░█ ▄ █▄█");
	println!("github.com/meigoc/auclick * version 1.0");
	println!("===========================================");
    let config: Config;
    if Path::new("config.ini").exists() {
        println!("Configuration file found. Recreate configuration? (y/n):");
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).expect("Error reading input");
        if answer.trim().to_lowercase() == "y" {
            config = create_config();
            config.save().expect("Error saving configuration to config.ini");
        } else {
            config = Config::load().expect("Error loading configuration");
        }
    } else {
        config = create_config();
        config.save().expect("Error saving configuration to config.ini");
    }
    println!("Configuration loaded: {:?}", config);
    let clicking = Arc::new(AtomicBool::new(false));
    let clicking_clone = clicking.clone();
    let config_clone = config.clone();
    thread::spawn(move || {
        let callback = move |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    let key_str = format!("{:?}", key);
                    if key_str == config_clone.start_key {
                        println!("Starting autoclicker");
                        clicking_clone.store(true, Ordering::SeqCst);
                    } else if key_str == config_clone.stop_key {
                        println!("Stopping autoclicker");
                        clicking_clone.store(false, Ordering::SeqCst);
                    }
                },
                EventType::ButtonPress(button) => {
                    let button_str = format!("{:?}", button);
                    if button_str == config_clone.start_key {
                        println!("Starting autoclicker");
                        clicking_clone.store(true, Ordering::SeqCst);
                    } else if button_str == config_clone.stop_key {
                        println!("Stopping autoclicker");
                        clicking_clone.store(false, Ordering::SeqCst);
                    }
                },
                _ => {}
            }
        };
        if let Err(e) = listen(callback) { eprintln!("Error listening: {:?}", e); }
    });
    let mut enigo = Enigo::new();
    let interval = Duration::from_secs_f64(1.0 / config.cps as f64);
    loop {
        if clicking.load(Ordering::SeqCst) {
            if config.click_key.starts_with("Key") {
                if config.click_key == "Key::A" {
                    enigo.key_click(Key::Layout('a'));
                } else if config.click_key == "Key::B" {
                    enigo.key_click(Key::Layout('b'));
                } else {
                    println!("Unknown key for clicking: {}", config.click_key);
                }
            } else {
                if config.click_key == "Left" {
                    enigo.mouse_click(MouseButton::Left);
                } else if config.click_key == "Right" {
                    enigo.mouse_click(MouseButton::Right);
                } else if config.click_key == "Middle" {
                    enigo.mouse_click(MouseButton::Middle);
                } else {
                    println!("Unknown mouse button: {}", config.click_key);
                }
            }
            thread::sleep(interval);
        } else {
            thread::sleep(Duration::from_millis(10));
        }
    }
}
