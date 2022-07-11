#![allow(unused_variables)]
#![allow(unused_must_use)]
#![allow(dead_code)]

use crate::nbd::NBDExport;
use clap::{Arg, arg, command, Command};
use std::sync::{Arc, RwLock};

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
            .arg(arg!(-f --force "Force requested size of the export").required(false))
        )
        .subcommand(
            Command::new("serve")
            .about("Serves export(s).")
            .arg(
                Arg::new("e")
                .long("export")
                .value_names(&["EXPORT", "DRIVER", "DRIVER_CFG"])
                .multiple_occurrences(true)
                .required(true)
            )
        )
        .subcommand(
            Command::new("destroy")
            .about("Destroys the export.")
            .arg(arg!([DRIVER] "Driver of the export").required(true))
            .arg(arg!([DRIVER_CFG] "Driver config of the export").required(true)),
            )
        .get_matches();

    let _ = match matches.subcommand() {
        Some(("init", sub_matches)) => init_export(
            sub_matches.value_of("size").unwrap(),
            sub_matches.value_of("DRIVER").unwrap(),
            sub_matches.value_of("DRIVER_CFG").unwrap(),
            sub_matches.is_present("force")
        ),

        Some(("serve", sub_matches)) => {
            let export_strs: Vec<&str> = sub_matches.values_of("e").unwrap().collect();
            println!("{:?}", export_strs);
            assert_eq!(export_strs.len() % 3, 0);

            let mut exports = Vec::<Arc<RwLock<NBDExport>>>::new();

            for i in 0..export_strs.len()/3 {
                let export = Arc::new(RwLock::new(NBDExport::new(
                            export_strs[i*3 + 0].to_string(),
                            String::from(export_strs[i*3 +1]),
                            String::from(export_strs[i*3 +2]),
                            )));
                exports.push(export);
            }
            serve_exports(exports)
        },

        Some(("destroy", sub_matches)) => destroy_export(
            sub_matches.value_of("DRIVER").unwrap(),
            sub_matches.value_of("DRIVER_CFG").unwrap(),
            ),
        _=> Ok(()),
    }.unwrap();
}
