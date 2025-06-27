# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AirCrate is a stream player and recorder built with Rust and the Bevy game engine. It features a vaporwave-inspired UI with a distinctive color palette and uses the Cobweb UI framework for reactive interfaces.

## Architecture

- **Main Application**: Built on Bevy 0.16.1 game engine using ECS (Entity Component System) architecture
- **UI Framework**: Uses bevy_cobweb and bevy_cobweb_ui for reactive UI components with hot reload support
- **Styling**: Custom color palette defined in `src/colors.rs` with vaporwave aesthetic
- **Scene Definition**: UI layouts defined in `.cob` files using Cobweb's declarative syntax

## Development Commands

### Build and Run
```bash
cargo run              # Run the application
cargo build            # Build the project
cargo build --release  # Build optimized release version
```

### Development Tools
```bash
cargo check            # Quick syntax/type checking
cargo clippy           # Linting
cargo fmt              # Code formatting
```

### Testing
```bash
cargo test             # Run all tests
```

## Key Components

- **Color System**: `AirCrateColors` struct provides a cohesive vaporwave color palette with methods for background, UI panels, highlights, and text colors
- **UI Layout**: Main UI built using Bevy's flexbox-style Node system with custom styling
- **Cobweb Integration**: Scene files (`.cob`) define reactive UI components with animation support

## File Structure

- `src/main.rs`: Application entry point and UI setup
- `src/colors.rs`: Centralized color palette definitions
- `assets/main.cob`: Cobweb scene definitions for UI components
- `Cargo.toml`: Dependencies include Bevy and Cobweb ecosystem

## UI Development Notes

- The application uses a purple background with contrasting UI panels
- Hot reload is enabled for Cobweb UI components during development
- Custom border radius and outline styling is applied throughout the interface
- Text colors are designed for readability against the dark vaporwave theme