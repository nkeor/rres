// Copyright (c) 2022 Namkhai B.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::process;

use anyhow::Context;
use simple_logger::SimpleLogger;

const USAGE: &str = "\
Usage: rres [options]

  -c, --card <card>       Specify a GPU (file existing in /dev/dri/, eg. card0)
  -m, --multi             Read all monitors. If this option is ommited, rres will
                          return the resolution of the first detected monitor
  -v, --verbose           Verbosity level. Can be specified multiple times, e.g. -vv
  -q, --quiet             Lower verbosity level. Opposite to -v
  -h, --help              Show this help message
  -g, --gamescope <mode>  Gamescope mode. Also supports FSR upscaling
                          Supported modes are native, ultra, quality, balanced and performance

Environment variables:

  RRES_DISPLAY=<index>      Select display in single mode (starting at 0)
  RRES_FORCE_RES=RESXxRESY  Force a specific resolution to be detected
  RRES_GAMESCOPE=<path>     Specify a gamescope binary for -g

Wine Virtual Desktop example:

  wine \"explorer /desktop=Game,$(./rres)\" game.exe

Gamescope usage:

  ./rres -g FSR_MODE -- GAMESCOPE_ARGS

  Example:
  ./rres -g ultra -- -f -- wine game.exe";

fn main() -> anyhow::Result<()> {
    // Settings
    let mut verbosity = log::LevelFilter::Warn;
    let mut multi = false;
    let mut card: Option<String> = None;
    let mut gamescope: Option<String> = None;
    let mut gamescope_args: Vec<String> = vec![];

    // Init logger
    SimpleLogger::new().with_level(verbosity).init()?;

    // Handle CLI
    {
        use lexopt::prelude::*;
        let mut parser = lexopt::Parser::from_env();

        while let Some(arg) = parser.next()? {
            match arg {
                Short('m') | Long("multi") => {
                    multi = true;
                }
                Short('c') | Long("card") => {
                    card = Some(parser.value()?.into_string().unwrap());
                }
                Short('h') | Long("help") => {
                    println!("{USAGE}");
                    process::exit(0);
                }
                Short('v') | Long("verbose") => {
                    verbosity = increment_loglevel(verbosity);
                }
                Short('q') | Long("quiet") => {
                    verbosity = decrement_loglevel(verbosity);
                }
                Short('g') | Long("gamescope") => {
                    gamescope = Some(parser.value()?.into_string().unwrap());
                }
                Value(val) => {
                    gamescope_args.push(val.to_string_lossy().to_string());
                    gamescope_args
                        .extend(parser.raw_args()?.map(|s| s.to_string_lossy().to_string()));
                }
                _ => return Err(arg.unexpected().into()),
            }
        }
    }

    if multi {
        // List every display
        let displays = rres::get_displays(card)?;

        for (i, mode) in displays.iter().enumerate() {
            let res = mode.size();
            println!("Display #{}: {}x{}", i, res.0, res.1);
        }

        return Ok(());
    }

    let res = rres::get_res_card(card)?;

    if let Some(fsr_mode) = gamescope {
        let mut gamescope_runner = rres::gamescope(res, &fsr_mode)?;

        gamescope_runner.extend(
            gamescope_args
                .iter()
                .map(|s| s.as_str().to_owned())
                .collect::<Vec<String>>(),
        );

        log::info!(
            "Running {} with args {:?}",
            &gamescope_runner[0],
            &gamescope_runner[1..]
        );

        let mut exec = process::Command::new(&gamescope_runner[0]);
        exec.args(&gamescope_runner[1..]);
        exec.spawn()
            .with_context(|| format!("failed to run {}", gamescope_runner[0]))?
            .wait()?;
    } else {
        println!("{}x{}", res.0, res.1);
    }

    Ok(())
}

/// Increase `log::LevelFilter` by one level
fn increment_loglevel(level: log::LevelFilter) -> log::LevelFilter {
    use log::LevelFilter::*;
    match level {
        Off => Error,
        Error => Warn,
        Warn => Info,
        Info => Debug,
        Debug | Trace => Trace,
    }
}

/// Decrease `log::LevelFilter` by one level
fn decrement_loglevel(level: log::LevelFilter) -> log::LevelFilter {
    use log::LevelFilter::*;
    match level {
        Off | Error => Off,
        Warn => Error,
        Info => Warn,
        Debug => Info,
        Trace => Debug,
    }
}
