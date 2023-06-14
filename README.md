# rres

> A xrandr replacement to gather display resolutions

_Recently under new maintainership, things subject to changes_

## Install

### from source
```sh
$ cargo install rres
```

### from AUR (Arch et all)
```sh
$ paru -S rres # or rres-git
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
                          Supported modes are native, ultra, quality, balanced and performance

Environment variables:

  RRES_DISPLAY=<index>      Select display in single mode (starting at 0)
  RRES_FORCE_RES=RESXxRESY  Force a specific resolution to be detected

Wine Virtual Desktop example:

  wine "explorer /desktop=Game,$(./rres)" game.exe

Gamescope usage:

  ./rres -g FSR_MODE -- GAMESCOPE_ARGS

  Example:
  ./rres -g ultra -- -f -- wine game.exe
```

## Changelog

All notable changes will be documented in the [CHANGELOG](./CHANGELOG.md)

## License

Licensed under the GPLv3 license.

Copyright (c) 2022 Namkhai B.
