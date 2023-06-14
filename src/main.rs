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

use std::env;
use std::fs;
use std::path;
use std::process;

use drm::control::{Device as ControlDevice, Mode};
use drm::Device;
use eyre::WrapErr;
use simple_logger::SimpleLogger;

mod fsr;

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

// Card handle
// Really just to get a raw file descriptor for `drm`
pub struct Card(std::fs::File);

impl std::os::unix::io::AsRawFd for Card {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.0.as_raw_fd()
    }
}

impl Card {
    pub fn open<P: AsRef<path::Path>>(path: P) -> Self {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        options.write(true);
        Card(options.open(path).unwrap())
    }
}

// Implement `drm` types
impl Device for Card {}
impl ControlDevice for Card {}

fn main() -> eyre::Result<()> {
    // Settings
    let mut verbosity = log::LevelFilter::Warn;
    let mut multi = false;
    let mut card: Option<String> = None;
    let mut gamescope: Option<String> = None;
    let mut gamescope_args: Vec<String> = vec![];

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
                    println!("{}", USAGE);
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

    let mut res = (0, 0);

    if let Ok(forced) = env::var("RRES_FORCE_RES") {
        if let Some((x, y)) = forced.split_once('x') {
            res = (x.parse()?, y.parse()?);
        } else {
            log::error!("failed to parse RRES_FORCE_RES");
            process::exit(1);
        }
    } else {
        // Init logger
        SimpleLogger::new().with_level(verbosity).init()?;

        // Store found displays
        let mut displays: Vec<Mode> = vec![];
        // Store the checked cards
        let mut cards: Vec<path::PathBuf> = vec![];

        if let Some(c) = card {
            // Open single card
            let mut file = path::PathBuf::from("/dev/dri/");
            file.push(&c);
            if !file.exists() || !c.starts_with("card") {
                return Err(eyre::eyre!("invalid card ({})", c));
            }
            cards.push(file);
        } else {
            // Open every card on the system
            for entry in fs::read_dir("/dev/dri/")? {
                let file = entry?;

                if let Some(name) = file.file_name().to_str() {
                    if name.starts_with("card") {
                        cards.push(file.path());
                    }
                }
            }
        }

        // Sort cards (card0, card1, card2...)
        cards.sort();

        // Read card list
        for file in cards {
            let gpu = Card::open(file);
            let info = gpu.get_driver()?;
            log::info!("Found GPU: {}", info.name().to_string_lossy());
            // Find displays
            match get_card_modes(gpu) {
                Ok(modes) => displays.extend_from_slice(&modes),
                Err(e) => log::error!("failed to read modes: {}", e),
            }
        }

        if displays.is_empty() {
            log::error!("found no display connected!");
            process::exit(1);
        }

        let selection: usize = env::var("RRES_DISPLAY")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .wrap_err("Failed to parse RRES_DISPLAY")?;
        if selection > displays.len() - 1 {
            return Err(eyre::eyre!("invalid display: {}", selection));
        }
        if multi {
            // List every display
            for (i, mode) in displays.iter().enumerate() {
                let res = mode.size();
                println!("Display #{}: {}x{}", i, res.0, res.1);
                return Ok(());
            }
        } else {
            // Print res of first display
            res = displays[selection].size();
        }
    }

    if let Some(fsr_mode) = gamescope {
        let gamescope_bin: String = env::var("RRES_GAMESCOPE").unwrap_or("gamescope".to_string());
        let mut gamescope_runner: Vec<&str> = vec![&gamescope_bin];

        let args;

        if fsr_mode.len() > 0 && fsr_mode.to_lowercase() != "native" {
            let fsr = match fsr::Fsr::try_from(fsr_mode.as_ref()) {
                Ok(m) => m,
                Err(_) => return Err(eyre::eyre!("invalid FSR mode: {}", fsr_mode)),
            };

            let fsr_res = fsr.generate(res);
            args = format!(
                "-W {} -H {} -U -w {} -h {}",
                res.0, res.1, fsr_res.0, fsr_res.1
            );
        } else {
            args = format!("-W {} -H {}", res.0, res.1);
        }

        gamescope_runner.extend(args.split(' '));
        gamescope_runner.extend(
            gamescope_args
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
        );

        log::info!(
            "Running {} with args {:?}",
            &gamescope_runner[0],
            &gamescope_runner[1..]
        );

        let mut exec = process::Command::new(gamescope_runner[0]);
        exec.args(&gamescope_runner[1..]);
        exec.spawn()
            .wrap_err_with(|| format!("failed to run {gamescope_bin}"))?
            .wait()?;
    } else {
        println!("{}x{}", res.0, res.1);
    }

    Ok(())
}

/// Get all the connected display's modes from a libdrm card.
pub fn get_card_modes<G: ControlDevice>(gpu: G) -> eyre::Result<Vec<Mode>> {
    let mut modes: Vec<Mode> = vec![];

    let resources = gpu
        .resource_handles()
        .wrap_err("failed to get resource handles")?;
    let connectors = resources.connectors();
    for handle in connectors {
        let connector = gpu
            .get_connector(*handle, false)
            .wrap_err("failed to get connector handle")?;
        if connector.state() == drm::control::connector::State::Connected {
            // Connected, get mode
            modes.push(get_connector_mode(&gpu, connector)?);
        }
    }
    Ok(modes)
}

/// Get current display mode from connector
///
/// Note: nVidia GPUs don't share the current encoder+crtc, so this function will report the
/// native display's resolution instead of the current resolution.
fn get_connector_mode<G: ControlDevice>(
    gpu: &G,
    connector: drm::control::connector::Info,
) -> eyre::Result<Mode> {
    if connector.state() != drm::control::connector::State::Connected {
        return Err(eyre::eyre!("Connector is disconnected"));
    }
    if let Some(encoder_handle) = connector.current_encoder() {
        // Get the encoder then crtc
        let encoder = gpu.get_encoder(encoder_handle)?;
        if let Some(crtc_handle) = encoder.crtc() {
            let crtc = gpu.get_crtc(crtc_handle).wrap_err("failed to get crtc")?;
            // Get current mode, and store it
            if let Some(current_mode) = crtc.mode() {
                log::info!(
                    "Found display: {:?}, {}x{}",
                    connector.interface(),
                    current_mode.size().0,
                    current_mode.size().1
                );
                return Ok(current_mode);
            }
        }
    }
    // nVidia GPUs don't expose the encoder (and thus neither the crtc)
    log::warn!(
        "Could not detect current mode for display {:?},",
        connector.interface()
    );
    log::warn!("reading native resolution");
    return Ok(connector.modes()[0]);
}

/// Increase `log::LevelFilter` by one level
fn increment_loglevel(level: log::LevelFilter) -> log::LevelFilter {
    use log::LevelFilter::*;
    match level {
        Off => Error,
        Error => Warn,
        Warn => Info,
        Info => Debug,
        Debug => Trace,
        Trace => Trace,
    }
}

/// Decrease `log::LevelFilter` by one level
fn decrement_loglevel(level: log::LevelFilter) -> log::LevelFilter {
    use log::LevelFilter::*;
    match level {
        Off => Off,
        Error => Off,
        Warn => Error,
        Info => Warn,
        Debug => Info,
        Trace => Debug,
    }
}
