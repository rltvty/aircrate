use bevy::prelude::*;
use bevy::window::{WindowMoved, WindowResized};
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
        // Only restore if we have saved position data that's not default
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
    }
    
    for event in window_resized_events.read() {
        window_manager.config.window.width = event.width;
        window_manager.config.window.height = event.height;
    }
}

pub fn save_window_state_on_exit(
    mut app_exit_events: EventReader<AppExit>,
    window_manager: Res<WindowManager>,
) {
    if !app_exit_events.is_empty() {
        app_exit_events.clear();
        window_manager.config.save();
    }
}