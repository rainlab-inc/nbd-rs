//TODO: Convert read & write functions to macros, so usage of these stuff will be simplified
// util::read_u16(clone_stream!(socket)) ---> read_u16!(socket)

#![macro_use]
#![allow(dead_code)]
use std::io::{Read, Write, Error};
use std::net::TcpStream;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Propagation {
   Guaranteed         = 255, // all layers we know reports completion, including external ones, to the final real disk
   Complete           = 127, // complete on our end, handed over to any external system if there is.
   InProgress         = 126, // background, but started
   Queued             = 125, // will do
   AppliedDifferently = 124, // Not exactly as requested (ex: fill-zeroes instead of trim)
   Unsure             =  32,  // execution attempted, but response from one layer taints response
   Redundant          =  24,  // skipped, because wasn't necessary / done already
   Noop               =  16,
   Ignored            =  15,
   Unsupported        =  14,
   // Failed      : u8 = 0,   // Instead of this, Result/Err should be used
}

/*
macro_rules! convert_ux_to_bytes {
    ($ty:ty, $num:expr) => {{
        let size = ::core::mem::size_of::<$ty>();
        println!("{:?}", size);
        let mut bytes = Vec::with_capacity(size as usize);
        for i in 0..size {
            bytes[i] = (($num >> (size - i - 1) * 8) & 0xff) as u8;
        }
        println!("{:?}", bytes);
        bytes
    }}
}
*/

macro_rules! read_x_bytes {
    ($ty:ty, $size:expr, $socket:expr) => {{
        assert_eq!($size, ::core::mem::size_of::<$ty>());
        let mut data = [0 as u8; $size];
        $socket.read(&mut data).expect("Error on reading client.");
        <$ty>::from_be_bytes(data)
    }};
}

macro_rules! read_string {
    ($size:expr, $socket:expr) => {{
        assert!($size > 0);
        let mut data = vec![0; $size as usize];
        $socket
            .read_exact(&mut data)
            .expect("Error on reading client.");
        <[u8]>::make_ascii_uppercase(&mut data[..]);
        String::from_utf8_lossy(&data).to_string()
    }};
}

macro_rules! write_x_bytes {
    ($ty:ty, $num:expr, $socket:expr) => {{
        assert_eq!(
            ::core::mem::size_of_val(&$num),
            ::core::mem::size_of::<$ty>()
        );
        let data = <$ty>::to_be_bytes($num);
        $socket.write(&data).expect("Error writing to client.");
    }};
}

macro_rules! write {
    ($buf:expr, $socket:expr) => {{
        $socket.write($buf).expect("Error on writing data")
    }};
}

/*
pub fn convert_u64_to_bytes(num: u64) -> Vec<u8> {
    convert_ux_to_bytes!(u64, num)
}
*/

pub fn read_u8(mut socket: &TcpStream) -> u8 {
    read_x_bytes!(u8, 1, socket)
}

pub fn read_u16(mut socket: &TcpStream) -> u16 {
    read_x_bytes!(u16, 2, socket)
}

pub fn read_u32(mut socket: &TcpStream) -> u32 {
    read_x_bytes!(u32, 4, socket)
}

pub fn read_u64(mut socket: &TcpStream) -> u64 {
    read_x_bytes!(u64, 8, socket)
}

pub fn read_string(size: usize, socket: &mut TcpStream) -> String {
    read_string!(size, socket)
}

pub fn write_u8(num: u8, socket: &mut TcpStream) {
    write_x_bytes!(u8, num, socket)
}

pub fn write_u16(num: u16, socket: &mut TcpStream) {
    write_x_bytes!(u16, num, socket)
}

pub fn write_u32(num: u32, socket: &mut TcpStream) {
    write_x_bytes!(u32, num, socket)
}

pub fn write_u64(num: u64, socket: &mut TcpStream) {
    write_x_bytes!(u64, num, socket)
}

pub struct AlignedBlockIter {
    pub from: usize,
    pub blksize: usize,
    pub to: usize,
}

impl Iterator for AlignedBlockIter {
    type Item = std::ops::Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.from == self.to {
            return None;
        }

        let start = self.from;
        let next_block_offset = self.blksize - (start % self.blksize);
        let mut to = start + next_block_offset;
        if to > self.to {
            to = self.to;
        }
        let len = to - start;
        let range = std::ops::Range { start, end: to };
        self.from = self.from + len;
        Some(range)
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use std::{
        fs::{remove_dir_all},
        ffi::{CString},
    };
    extern crate libc;

    pub struct TempFolder {
        pub path: String
    }

    impl TempFolder {
        pub fn new() -> TempFolder {
            let tmp_dir = std::env::temp_dir();
            let ptr = CString::new(format!("{}/nbd-rs-tmp-XXXXXX", tmp_dir.display()))
                        .unwrap()
                        .into_raw();
            unsafe {
                let folder = libc::mkdtemp(ptr);
                if folder.is_null() {
                    std::panic::panic_any(Error::last_os_error());
                }
            }
            let path = unsafe { CString::from_raw(ptr) }.into_string().unwrap();
            TempFolder { path: path }
        }
    }

    impl Drop for TempFolder {
        fn drop(&mut self) {
            remove_dir_all(self.path.clone());
        }
    }
}
