# Monitor Management in Bevy - Research Notes

## Current Implementation (Simple Index-Based)

We currently use a simple monitor index system:
- Save `monitor_index` (0, 1, 2, etc.) to `config.toml`
- Restore window using `WindowPosition::Centered(MonitorSelection::Index(monitor_idx))`
- Then move to exact saved coordinates with `WindowPosition::At()`

### Pros:
- ✅ Simple and works reliably
- ✅ Handles basic multi-monitor setups
- ✅ Low complexity

### Cons:
- ❌ Monitor indices can change if monitors are unplugged/replugged
- ❌ No human-readable monitor identification

## Advanced Implementation Options (Future Enhancement)

### 1. Monitor Component API (Bevy 0.15+)

Bevy provides a `Monitor` component for accessing monitor information:

```rust
use bevy::window::Monitor;

fn enumerate_monitors(monitors: Query<&Monitor>) {
    for (index, monitor) in monitors.iter().enumerate() {
        println!("Monitor {}: {}", 
                 index, 
                 monitor.name.as_deref().unwrap_or("Unknown"));
        println!("  Size: {}x{}", monitor.physical_width, monitor.physical_height);
        println!("  Position: {:?}", monitor.physical_position);
        println!("  Scale Factor: {}", monitor.scale_factor);
    }
}
```

### 2. Enhanced MonitorSelection

The `MonitorSelection` enum provides multiple targeting options:

```rust
pub enum MonitorSelection {
    Current,           // Current monitor of the window
    Primary,          // Primary monitor of the system  
    Index(usize),     // Monitor with specified index (our current approach)
    Entity(Entity),   // Specific Monitor entity (most robust)
}
```

### 3. Robust Monitor Mapping Approach

For future implementation, consider:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MonitorInfo {
    pub name: String,
    pub physical_size: (u32, u32),
    pub position: (i32, i32),
    pub entity_id: Option<u64>, // For Entity-based selection
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowConfig {
    pub x: i32,
    pub y: i32,
    pub width: f32,
    pub height: f32,
    pub monitor_info: Option<MonitorInfo>, // More robust than just index
}
```

### 4. Monitor Detection System

```rust
fn build_monitor_map(
    monitors: Query<(Entity, &Monitor)>
) -> HashMap<String, (usize, Entity)> {
    monitors
        .iter()
        .enumerate()
        .map(|(idx, (entity, monitor))| {
            let name = monitor.name.as_deref().unwrap_or("Unknown").to_string();
            (name, (idx, entity))
        })
        .collect()
}

fn restore_to_monitor_by_name(
    monitor_name: &str,
    monitor_map: &HashMap<String, (usize, Entity)>
) -> MonitorSelection {
    if let Some((_, entity)) = monitor_map.get(monitor_name) {
        MonitorSelection::Entity(*entity)
    } else {
        MonitorSelection::Primary // Fallback
    }
}
```

## Recommended Future Enhancement Steps

1. **Phase 1**: Implement monitor enumeration to log available monitors
2. **Phase 2**: Store monitor name alongside index in config
3. **Phase 3**: Implement fallback logic (try by name, fall back to index, then primary)
4. **Phase 4**: Use Entity-based selection for maximum robustness

## Implementation Challenges Discovered

1. **Monitor API Complexity**: The Monitor component system requires careful ECS integration
2. **Platform Differences**: Monitor APIs may behave differently across Windows/macOS/Linux
3. **Hot-plugging**: Need to handle monitors being added/removed during runtime
4. **Entity Persistence**: Monitor entities don't persist across app restarts

## Code Locations

- Window restoration: `src/main.rs` (initial monitor selection)
- Position tracking: `src/window_manager.rs` (monitor detection heuristics)
- Config storage: `src/config.rs` (monitor_index field)

## References

- [Bevy Monitor Component](https://docs.rs/bevy/latest/bevy/window/struct.Monitor.html)
- [MonitorSelection Enum](https://docs.rs/bevy/latest/bevy/prelude/enum.MonitorSelection.html)
- [WindowPosition Enum](https://docs.rs/bevy/latest/bevy/prelude/enum.WindowPosition.html)

---

*Last updated: 2025-06-27*
*Current approach: Simple index-based with position coordinates*