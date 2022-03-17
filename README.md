# rres

> A xrandr replacement to gather display resolutions

_Recently under new maintainership, things subject to changes_

## Install

```
cargo install rres
```

## Usage

```
$ rres -h
Usage: rres [options]

  -c, --card <card>       Specify a GPU (file existing in /dev/dri/, eg. card0)
  -m, --multi             Read all monitors. If this option is ommited, rres will
                          return the resolution of the first detected monitor
  -v, --verbose           Verbosity level. Can be specified multiple times, e.g. -vv
  -q, --quiet             Lower verbosity level. Opposite to -v
  -h, --help              Show this help message
  -g, --gamescope <mode>  Gamescope mode. Also supports FSR upscaling
                          Supported modes are none, ultra, quality, balanced and performance

Environment variables:

  RRES_DISPLAY=<index>      Select display in single mode (starting at 0)
  RRES_FORCE_RES=RESXxRESY  Force a specific resolution to be detected

Wine Virtual Desktop example:

  wine "explorer /desktop=Game,$(./rres)" game.exe

Gamescope example:

  gamescope $(./rres -g ultra) -- wine game.exe
```

## Changelog

All notable changes will be documented in the [CHANGELOG](./CHANGELOG.md)

## License

Licensed under the GPLv3 license.

Copyright (c) 2022 Namkhai B.
