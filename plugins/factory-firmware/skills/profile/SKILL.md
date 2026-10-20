---
name: profile
description: Firmware profile rules for Pedro's Factory - read by /factory:init and /factory:build, not for direct use.
user-invocable: false
---
# Profile: firmware (PlatformIO)

## INIT
Questions (skip what platformio.ini already answers):
- Target environments (`[env:...]` in platformio.ini) and which one is the native test env.
- Simulator: Wokwi (needs `wokwi-cli` and a `WOKWI_CLI_TOKEN`), or none.
- Dev boards that may be flashed automatically when connected: name, `pio device list` serial
  port pattern or USB VID:PID, and the env that targets each. Any board not on this list is
  never flashed by the pipeline.
- What the serial output should show when the firmware works (boot banner, status lines).

Record in stack.md under `## firmware`: envs, native test env, simulator, the SAFE DEV BOARDS
list, the expected serial output, and the gates below.
Always escalate: flashing any board not on the safe list, bootloader or fuse settings, partition
tables, OTA/update code, power and clock configuration, anything driving motors or mains.

MCP: none. Secrets: if Wokwi is used, `WOKWI_CLI_TOKEN` follows the token steps in
`/factory:init` (create it at wokwi.com/dashboard/ci).
Tooling check: `pio --version` (install: `pipx install platformio`), and `wokwi-cli --version`
when the simulator is used.

## BUILD
Builder rules to add to every brief: keep hardware access behind small interfaces so logic is
testable in the native env; every module gets a Unity test under `test/` that runs in the native
env; no blocking delays in interrupt handlers; mark shared ISR variables `volatile` and guard
them.

## GATES
`pio run` for every target env, `pio test -e <native env>`, `pio check --fail-on-defect high`.

## TESTERS
- `factory-firmware:fw-tester`: pass ONLY ENVS, NATIVE_ENV, SIMULATOR (with its scenario file if
  any), SAFE_BOARDS (from stack.md), EXPECTED_OUTPUT, ACCEPTANCE_CRITERIA and REPORT
  (`round-<n>/fw.md`).

## REVIEWERS
- `factory-firmware:reviewer-firmware` (REPORT `round-<n>/firmware.md`).

## ESCALATE
Risk tags: `flash-unsafe-board`, `bootloader`, `power`, `ota`.
