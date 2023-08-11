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
use std::os;
use std::path;

use anyhow::Context;
use drm::control::{Device as ControlDevice, Mode};
use drm::Device;

mod fsr;

// Card handle
// Really just to get a file descriptor for `drm`
struct Card(std::fs::File);

impl os::fd::AsFd for Card {
    fn as_fd(&self) -> os::fd::BorrowedFd<'_> {
        self.0.as_fd()
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

/// Build FSR arguments for gamescope
pub fn gamescope(res: (u16, u16), fsr_mode: &str) -> anyhow::Result<Vec<String>> {
    let gamescope_bin: String = env::var("RRES_GAMESCOPE").unwrap_or("gamescope".to_string());
    let mut gamescope_runner: Vec<String> = vec![gamescope_bin];

    let args = if !fsr_mode.is_empty() && fsr_mode.to_lowercase() != "native" {
        let Ok(fsr) = fsr::Fsr::try_from(fsr_mode) else {
            return Err(anyhow::anyhow!("invalid FSR mode: {}", fsr_mode));
        };

        let fsr_res = fsr.generate(res);
        format!(
            "-W {} -H {} -U -w {} -h {}",
            res.0, res.1, fsr_res.0, fsr_res.1
        )
    } else {
        format!("-W {} -H {}", res.0, res.1)
    };

    gamescope_runner.extend(args.split(' ').map(|s| s.to_owned()));

    Ok(gamescope_runner)
}

/// Get all the displays from the system or selected card
pub fn get_displays(card: Option<String>) -> anyhow::Result<Vec<Mode>> {
    // Store found displays
    let mut displays: Vec<Mode> = vec![];
    // Store the checked cards
    let mut cards: Vec<path::PathBuf> = vec![];

    if let Some(c) = card {
        // Open single card
        let mut file = path::PathBuf::from("/dev/dri/");
        file.push(&c);
        if !file.exists() || !c.starts_with("card") {
            return Err(anyhow::anyhow!("invalid card ({c})"));
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
        log::debug!("Found GPU: {}", info.name().to_string_lossy());
        // Find displays
        match get_card_modes(&gpu) {
            Ok(modes) => displays.extend_from_slice(&modes),
            Err(e) => log::error!("failed to read modes: {e}"),
        }
    }

    Ok(displays)
}

/// Get the resolution from first display
pub fn get_res() -> anyhow::Result<(u16, u16)> {
    get_res_card(None)
}

/// Get the resolution from the first display of the selected card
pub fn get_res_card(card: Option<String>) -> anyhow::Result<(u16, u16)> {
    let res;

    if let Ok(forced) = env::var("RRES_FORCE_RES") {
        if let Some((x, y)) = forced.split_once('x') {
            res = (x.parse()?, y.parse()?);
        } else {
            return Err(anyhow::anyhow!("failed to parse RRES_FORCE_RES"));
        }
    } else {
        let displays = get_displays(card)?;

        let selection: usize = env::var("RRES_DISPLAY")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .context("Failed to parse RRES_DISPLAY")?;

        if selection > displays.len() - 1 {
            return Err(anyhow::anyhow!("invalid display: {}", selection));
        }

        res = displays[selection].size();
    }

    Ok(res)
}

/// Get all the connected display's modes from a libdrm card.
pub fn get_card_modes<G: ControlDevice>(gpu: &G) -> anyhow::Result<Vec<Mode>> {
    let mut modes: Vec<Mode> = vec![];

    let resources = gpu
        .resource_handles()
        .context("failed to get resource handles")?;
    let connectors = resources.connectors();
    for handle in connectors {
        let connector = gpu
            .get_connector(*handle, false)
            .context("failed to get connector handle")?;
        if connector.state() == drm::control::connector::State::Connected {
            // Connected, get mode
            modes.push(get_connector_mode(gpu, &connector)?);
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
    connector: &drm::control::connector::Info,
) -> anyhow::Result<Mode> {
    if connector.state() != drm::control::connector::State::Connected {
        return Err(anyhow::anyhow!("Connector is disconnected"));
    }
    if let Some(encoder_handle) = connector.current_encoder() {
        // Get the encoder then crtc
        let encoder = gpu.get_encoder(encoder_handle)?;
        if let Some(crtc_handle) = encoder.crtc() {
            let crtc = gpu.get_crtc(crtc_handle).context("failed to get crtc")?;
            // Get current mode, and store it
            if let Some(current_mode) = crtc.mode() {
                log::debug!(
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
