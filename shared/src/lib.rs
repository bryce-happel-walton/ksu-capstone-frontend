pub const ESP_DATA_DIR: &'static str = "ws://192.168.4.1/data";
pub const WEB_SOCKET_DIR: &'static str = "web_socket";

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

impl TestData {
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

    pub fn str_from_chars(field: &[std::ffi::c_char]) -> &str {
        let bytes = unsafe { std::slice::from_raw_parts(field.as_ptr() as *const u8, field.len()) };
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..len]).unwrap_or("")
    }
}
