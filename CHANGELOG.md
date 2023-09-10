# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [v0.1.5] - 2023-09-10

### Changed

* Updated author, homepage & repo

## [0.1.4] - 2023-08-09

### Breaking changes

* Reworked gamescope integration: `rres -g FSR_MODE -- GAMESCOPE_ARGS`

### Changed

* Replaced `-g none` with `-g native`
* rres is now also a library

## [0.1.3] - 2022-10-27

### Updated

* Updated `drm` to v0.7, `simple_logger` to v4.0
* Moved homepage to [SourceHut](https://sr.ht/~f9/rres)

## [0.1.2] - 2022-03-17

### Added

* `-g, --gamescope <mode>` to enable on-the-fly gamescope FSR support

### Chore

* Updated dependencies

## [0.1.1] - 2022-01-25

### Added

* `RRES_FORCE_RES` env variable

### Changed

* Improved error messages

## [0.1.0] - 2021-12-11

Initial release
