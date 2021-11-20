#![allow(unused_variables)]
use std::{
    collections::{HashMap},
};

use clap::{App, Arg, crate_authors, crate_version};

use crate::{
    nbd::{NBDExportConfig, NBDServer},
};

use log;
use env_logger;

// https://github.com/NetworkBlockDevice/nbd/blob/master/nbd-server.c#L2362-L2468
// NBD_OPT_GO | NBD_OPT_INFO: https://github.com/NetworkBlockDevice/nbd/blob/master/nbd-server.c#L2276-L2353

mod object;
mod block;
mod util;
mod nbd;

fn main() {
    env_logger::init();
    log::trace!("Parsing arguments");
    let matches = App::new("nbd-rs")
        .about("NBD Server written in Rust.")
        .author(crate_authors!())
        .version(crate_version!())
        .arg(
            Arg::with_name("export")
                .short("e")
                .long("export")
                .takes_value(true)
                .value_names(&["export_name", "driver", "conn_str"])
                .required(true)
                .multiple(true)
                .number_of_values(3)
                .long_help(
"USAGE:
[-e | --export EXPORT_NAME; DRIVER (raw, sharded); CONN_STR]...
Sets the export(s) via `export` argument. Must be used at least once."),
        )
        .get_matches();
    let vals: Vec<String> = matches.values_of("export")
        .unwrap()
        .map(|val| val.to_string())
        .collect();
    let mut exports = HashMap::<String, NBDExportConfig>::new();
    for i in 0..(matches.occurrences_of("export") as usize) {
        exports.insert(vals[i * 3].clone(),
            NBDExportConfig::new(
                vals[i * 3].clone(),
                vals[i * 3 + 1].clone(),
                vals[i * 3 + 2].clone()
            )
        );
    }
    let mut server = NBDServer::new("0.0.0.0".to_string(), 10809, exports);
    server.listen();
}
