pub const ESP_IP: &'static str = "192.168.4.1";

pub const SERVER_IP: &'static str = "127.0.0.1";
pub const SERVER_WS_TEST_DATA_DIR: &'static str = "test_data";
pub const SERVER_WS_IMAGE_STREAM_DIR: &'static str = "image_stream";

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

pub fn cstr_to_str(bytes: &[u8]) -> &str {
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..len]).unwrap_or("")
}

impl InputData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let ptr = self as *const Self as *const u8;
        unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of::<Self>()) }.to_vec()
    }
}

impl RadarPayload {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == std::mem::size_of::<Self>() {
            Some(unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const Self) })
        } else {
            None
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let ptr = self as *const Self as *const u8;
        unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of::<Self>()) }.to_vec()
    }
}

pub fn str_from_chars(field: &[std::ffi::c_char]) -> &str {
    let bytes = unsafe { std::slice::from_raw_parts(field.as_ptr() as *const u8, field.len()) };
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..len]).unwrap_or("")
}
