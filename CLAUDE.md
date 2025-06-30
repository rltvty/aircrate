# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## User Preferences & Guidelines

### Code Changes
- **ALWAYS ASK BEFORE MAKING CHANGES** - Do not take actions without specifically asking the user first
- When there are multiple approaches, present options and let the user decide
- Wait for explicit permission before modifying code

### Project Guidelines
- Do not create files unless absolutely necessary for achieving the goal
- Always prefer editing existing files over creating new ones
- Create documentation and README files to explain complex systems and help future development

## Project Overview

AirCrate is a stream player and recorder built with Rust. **CURRENT ARCHITECTURE**: Tokio-first with Bevy UI (not the original Cobweb-based design).

## Current Architecture (Updated)

- **Runtime**: Tokio multi-threaded async runtime manages all streaming tasks
- **Main Thread**: Bevy runs on main thread for UI (required for macOS compatibility)
- **Audio Pipeline**: HTTP streaming → Tokio tasks → dedicated audio thread → Rodio/Symphonia → speakers
- **Communication**: tokio::sync channels (mpsc for data, watch for UI state)

## Current Status

### Working Components
- ✅ Real HTTP streaming from Flux/Clubsandwich channel 
- ✅ AAC audio decoding with Symphonia
- ✅ Real-time audio playback with Rodio (user confirmed audio is working)
- ✅ Tokio async task coordination
- ✅ Bevy UI with real-time state updates
- ✅ Error handling and automatic reconnection

### Next Possible Steps (ask user which to do)
- Recording with overlap buffers  
- UI controls (start/stop/volume buttons)
- Real track info from API (currently using mock data)
- Channel dropdown selection

## Development Commands

### Build and Run
```bash
cargo run              # Run the application
cargo check            # Quick syntax/type checking
```

## Key Technical Details

### Stream Info
- Current URL: `https://fluxmusic.api.radiosphere.io/channels/clubsandwich/stream.aac?quality=10`
- Channel API: `https://fluxmusic.api.radiosphere.io/channels`
- Format: AAC 320kbps
- Audio working: User confirmed they can hear the music

### Architecture Flow
```
HTTP Stream → Tokio Task → Audio Buffer → Audio Thread → Rodio → Speakers
     ↓              ↓           ↓           ↓
Real AAC chunks  Accumulate   Decode    Playback
                              w/Symphonia
```

### File Structure (Current)
- `src/main.rs`: Complete tokio-first application with all components
- `Cargo.toml`: Dependencies for tokio, reqwest, rodio, symphonia, bevy
- No other source files currently (all in main.rs for simplicity)