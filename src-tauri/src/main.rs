// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Main entry point
//
// LEGAL NOTICE:
// This project is NOT affiliated with Motrix or any other download manager.
// This is an independent, open-source project licensed under AGPL-3.0.
//
// No telemetry. No tracking. No ads. Local-only state.

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gosh_transfer::run()
}
