---
name: profile
description: Robotics profile rules for Pedro's Factory - read by /factory:init and /factory:build, not for direct use.
user-invocable: false
---
# Profile: robotics (ROS 2 + Gazebo)

## INIT
Questions (skip what the repo already answers):
- ROS 2 distro (for example jazzy or humble) and how it's sourced (`/opt/ros/<distro>/setup.bash`).
- Workspace layout: the colcon workspace root and packages that belong to this project.
- Simulation: the launch command that starts Gazebo headless with the robot and world
  (for example `ros2 launch <pkg> sim.launch.py headless:=true`), and how long it takes to come up.
- Which topics, services and actions show that the robot behaves (odometry, joint states,
  goal results, diagnostics).
- Is real hardware ever connected to this machine? Which launch files or parameters drive it?

Record in stack.md under `## robotics`: distro and setup command, workspace root, build
command (`colcon build --symlink-install`), test command (`colcon test && colcon test-result
--verbose`), the sim launch command, observable topics, and the hardware launch files.
Always escalate: anything that launches or configures real hardware, safety limits (velocity,
torque, workspace bounds), e-stop and watchdog code, firmware flashing, calibration files.

MCP: none required; agents use the `ros2`, `colcon` and `gz` CLIs.
Tooling check: `ros2 --help`, `colcon --help`, `gz sim --help` (after sourcing the distro).
If missing, point to the official ROS 2 and Gazebo install pages for the user's OS instead of
installing anything.

## BUILD
Builder rules to add to every brief: source the distro setup first; never launch hardware
launch files; keep units in SI and frames per REP 103/105; put new limits in parameters, not
literals; every new node gets a launch test or unit test.

## GATES
`colcon build --symlink-install`, then the test command from stack.md. Source the workspace
(`install/setup.bash`) before tests.

## TESTERS
- `factory-robotics:sim-tester`: pass ONLY SETUP (the source commands), LAUNCH (the sim launch
  command), SCENARIO (what the robot should do, in plain words, plus any goal to send),
  OBSERVE (topics/services/actions to watch), ACCEPTANCE_CRITERIA, TIMEOUT and REPORT
  (`round-<n>/sim.md`).

## REVIEWERS
- `factory-robotics:reviewer-safety` (REPORT `round-<n>/safety.md`).

## ESCALATE
Risk tags: `hardware`, `safety-limits`, `estop`, `calibration`.
