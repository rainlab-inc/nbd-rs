#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_must_use)]
#![allow(dead_code)]

use clap::{Arg, arg, command, Command, crate_authors, crate_version};

mod object;
mod block;
mod util;
mod nbd;
mod core;
use crate::core::*;


fn main() {
    env_logger::init();
    log::trace!("Parsing arguments");
    let matches = command!()
        .subcommand(
            Command::new("init")
            .about("Initializes the export.")
            .arg(arg!(-s --size <SIZE> "Requested size of the export").required(true))
            .arg(arg!([DRIVER] "Driver of the export").required(true))
            .arg(arg!([DRIVER_CFG] "Driver config of the export").required(true))
            )
        .subcommand(
            Command::new("serve")
            .about("Serves the export.")
            .arg(arg!([EXPORT] "Name of the export").required(true))
            .arg(arg!([DRIVER] "Driver of the export").required(true))
            .arg(arg!([DRIVER_CFG] "Driver config of the export").required(true)),
            )
        .get_matches();



    let _ = match matches.subcommand() {
        Some(("init", sub_matches)) => export_init(
            sub_matches.value_of("size").unwrap(),
            sub_matches.value_of("DRIVER").unwrap(),
            sub_matches.value_of("DRIVER_CFG").unwrap(),
            ),
            Some(("serve", sub_matches)) => export_serve(
                sub_matches.value_of("EXPORT").unwrap(),
                sub_matches.value_of("DRIVER").unwrap(),
                sub_matches.value_of("DRIVER_CFG").unwrap(),
            ),
            _=> Ok(()),
    }.unwrap();
}

