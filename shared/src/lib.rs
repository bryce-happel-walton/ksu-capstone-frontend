use serde::{Deserialize, Serialize};

pub const ESP_DATA_DIR: &'static str = "http://192.168.4.1/data";
pub const WEB_SOCKET_DIR: &'static str = "web_socket";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct EspData {
    pub hello: String,
    pub beep: u32,
    pub boop: bool,
}
