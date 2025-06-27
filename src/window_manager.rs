use bevy::prelude::*;
use bevy::window::{WindowMoved, WindowResized, WindowCloseRequested};
use crate::config::AppConfig;

#[derive(Resource)]
pub struct WindowManager {
    pub config: AppConfig,
    has_restored_position: bool,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            config: AppConfig::load(),
            has_restored_position: false,
        }
    }
}

pub fn restore_window_position(
    mut window_manager: ResMut<WindowManager>,
    mut windows: Query<&mut Window>,
) {
    if window_manager.has_restored_position {
        return;
    }

    if let Ok(mut window) = windows.single_mut() {
        // The window should already be centered on the correct monitor from main.rs
        // Now move it to the exact saved position if we have saved coordinates
        if window_manager.config.window.x != 100 || window_manager.config.window.y != 100 {
            window.position = WindowPosition::At(IVec2::new(
                window_manager.config.window.x,
                window_manager.config.window.y,
            ));
        }
        
        window_manager.has_restored_position = true;
    }
}

pub fn track_window_changes(
    mut window_moved_events: EventReader<WindowMoved>,
    mut window_resized_events: EventReader<WindowResized>,
    mut window_manager: ResMut<WindowManager>,
    _windows: Query<&Window>,
) {
    for event in window_moved_events.read() {
        window_manager.config.window.x = event.position.x;
        window_manager.config.window.y = event.position.y;
        
        // Determine which monitor the window is on using improved heuristics
        window_manager.config.window.monitor_index = detect_monitor_from_position(event.position);
    }
    
    for event in window_resized_events.read() {
        window_manager.config.window.width = event.width;
        window_manager.config.window.height = event.height;
    }
}

fn detect_monitor_from_position(position: IVec2) -> Option<usize> {
    // Improved heuristics for common monitor setups
    // Most common resolutions for primary monitors: 1920x1080, 2560x1440, 3440x1440, etc.
    
    // If X position is negative, likely on monitor to the left
    if position.x < -100 {
        return Some(1);
    }
    
    // If X position is beyond typical primary monitor widths, likely on secondary monitor
    if position.x > 1920 && position.x > 0 {
        // Could be on a secondary monitor to the right
        // Common secondary monitor positions start around primary width
        Some(1)
    } else if position.x >= 0 && position.x <= 1920 {
        // Likely on primary monitor (assuming 1920px or smaller primary)
        Some(0)
    } else if position.x > 2560 {
        // Definitely on secondary monitor for larger primaries
        Some(1)
    } else {
        // Default to primary if unsure
        Some(0)
    }
}

pub fn save_window_state_on_close(
    mut window_close_events: EventReader<WindowCloseRequested>,
    window_manager: Res<WindowManager>,
) {
    if !window_close_events.is_empty() {
        window_close_events.clear();
        window_manager.config.save();
    }
}