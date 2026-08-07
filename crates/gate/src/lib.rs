use pystral_runtime::{RuntimeRequest, RuntimeResponse};
use pystral_core::history::HistoryManager;
use serde::{Deserialize, Serialize};

pub mod render;
pub mod worker;

#[cfg(test)]
mod strict_log_test;
#[cfg(test)]
mod character_test;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope<T> {
    pub seq: u64,
    pub msg: T,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReliableInput {
    Msg(Envelope<WorkerInput>),
    Watermark(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReliableOutput {
    Msg(Box<Envelope<WorkerOutput>>),
    Watermark(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerInput {
    Log(String),
    ResetLog,
    RuntimeRequest(RuntimeRequest),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerOutput {
    LogUpdate { messages: Vec<String>, total_errors: u32 },
    RuntimeResponse(Box<RuntimeResponse>),
}

pub enum AppCommand {
    SetHistoryIndex(u32),
    TogglePlayLog,
    TogglePlayAnimations,
    SetDebugMode(bool),
    UpdateHistory(Box<HistoryManager>),
    CameraNav(String),
}

#[cfg(any(test, debug_assertions))]
pub fn load_test_assets() -> (String, Vec<u8>, u32) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let web = root.join("web");
    
    let atlas = std::fs::read_to_string(web.join("atlas.json")).expect("Failed to load atlas.json for test");
    let spritesheet = std::fs::read(web.join("spritesheet.png")).expect("Failed to load spritesheet.png for test");
    
    let decoder = png::Decoder::new(std::io::Cursor::new(&spritesheet));
    let mut reader = decoder.read_info().expect("Failed to read spritesheet info for test");
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).expect("Failed to read spritesheet frame for test");
    
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => {
            let mut rgba = Vec::with_capacity((info.width * info.height * 4) as usize);
            for chunk in buf.chunks_exact(3) {
                rgba.push(chunk[0]);
                rgba.push(chunk[1]);
                rgba.push(chunk[2]);
                rgba.push(255);
            }
            rgba
        }
        _ => panic!("Unsupported spritesheet color type in test: {:?}", info.color_type),
    };
    
    (atlas, rgba, info.width)
}
