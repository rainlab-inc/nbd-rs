//TODO: Convert read & write functions to macros, so usage of these stuff will be simplified
// util::read_u16(clone_stream!(socket)) ---> read_u16!(socket)

#![macro_use]
#![allow(dead_code)]
use std::io::{Read, Write};
use std::net::TcpStream;

macro_rules! clone_stream {
    ($stream:expr) => {
        ($stream.try_clone().expect("Err on cloning stream"))
    };
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
        let mut data = Vec::with_capacity($size);
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

pub fn read_u8(mut socket: TcpStream) -> u8 {
    read_x_bytes!(u8, 1, socket)
}

pub fn read_u16(mut socket: TcpStream) -> u16 {
    read_x_bytes!(u16, 2, socket)
}

pub fn read_u32(mut socket: TcpStream) -> u32 {
    read_x_bytes!(u32, 4, socket)
}

pub fn read_u64(mut socket: TcpStream) -> u64 {
    read_x_bytes!(u64, 8, socket)
}

pub fn read_string(size: usize, mut socket: TcpStream) -> String {
    read_string!(size, socket)
}

pub fn write_u8(num: u8, mut socket: TcpStream) {
    write_x_bytes!(u8, num, socket)
}

pub fn write_u16(num: u16, mut socket: TcpStream) {
    write_x_bytes!(u16, num, socket)
}

pub fn write_u32(num: u32, mut socket: TcpStream) {
    write_x_bytes!(u32, num, socket)
}

pub fn write_u64(num: u64, mut socket: TcpStream) {
    write_x_bytes!(u64, num, socket)
}
