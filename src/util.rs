#![macro_use]
#![allow(dead_code)]
use std::io::{Read, Write};
use std::net::TcpStream;

macro_rules! clone_stream {
    ($stream:expr) => {(
        //assert_eq!(::std::any::type_name_of_val($stream), "std::net::TcpStream"); Unstable for rust v1.47
        $stream.try_clone().expect("Err on cloning stream")
    )};
}

macro_rules! read_x_bytes {
    ($ty:ty, $size:expr, $socket:expr) => {{
        assert_eq!($size, ::core::mem::size_of::<$ty>());
        let mut data = [0 as u8; $size];
        $socket.read(&mut data).expect("Error on reading client.");
        <$ty>::from_be_bytes(data)
    }};
}

macro_rules! write_x_bytes {
    ($ty:ty, $num:expr, $socket:expr) => {{
        assert_eq!(::core::mem::size_of_val(&$num), ::core::mem::size_of::<$ty>());
        let data = <$ty>::to_be_bytes($num);
        $socket.write(&data).expect("Error writing to client.");
    }};
}

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
