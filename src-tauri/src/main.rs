// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Main entry point

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gosh_transfer::run()
}
