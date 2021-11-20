#![allow(unused_variables)]

use std::{
    net::{TcpStream},
};

use crate::{
    block::{BlockStorage, block_storage_with_config},
};

pub struct NBDSession {
    pub socket: TcpStream,
    pub flags: [bool; 2],
    pub structured_reply: bool,
    pub driver: Option<Box<dyn BlockStorage>>,
    pub driver_name: String,
    // TODO: contexts: list of active contexts with attached metadata_context_ids
    pub metadata_context_id: u32,
    //request: Option<NBDRequest>,
    //option: Option<NBDOption>, // addr: SocketAddr,
                               // socket
                               // inputQueue
                               // remote
}

impl NBDSession {
    pub fn new(
        socket: TcpStream,
        flags: [bool; 2],
        structured_reply: bool,
        driver_name: String,
        image_name: String,
        metadata_context_id: u32,
        storage_config: String
    ) -> NBDSession {
        let mut session = NBDSession {
            socket: socket,
            flags: flags,
            structured_reply: structured_reply,
            driver: None,
            metadata_context_id: metadata_context_id,
            driver_name: driver_name.clone()
        };
        if driver_name.as_str() != "" {
            let driver = block_storage_with_config(image_name, driver_name, storage_config).unwrap();
            session.driver = Some(driver);
        }
        session
    }

    pub fn set_structured_reply(self) -> Self {
        NBDSession {
            structured_reply: true,
            ..self
        }
    }

    pub fn set_metadata_context_id(self, metadata_context_id: u32) -> Self {
        NBDSession {
            metadata_context_id: metadata_context_id,
            ..self
        }
    }

}
