// Back away from the cursor when it comes close, drift home when it
// doesn't. Shows the cursor inputs, including why `has_cursor` exists:
// `cursor_x` / `cursor_y` are meaningless when it is false, so reading
// them unconditionally would make the character flee a stale point.
//
//   [characters.behavior.params]
//   radius = 200.0
//   speed = 180.0

if !state.contains("home_x") {
    state.home_x = x;
}

let fleeing = false;

if has_cursor {
    let centre = x + w / 2.0;
    let gap = centre - cursor_x;
    let dist = if gap < 0.0 { -gap } else { gap };

    if dist < params.radius {
        let away = if gap < 0.0 { -1.0 } else { 1.0 };
        x += away * params.speed * dt;
        fleeing = true;
    }
}

if !fleeing {
    // Ease back to where it started, slowly.
    x += (state.home_x - x) * 0.5 * dt;
}

// Never leave the desktop.
if x < bounds_min_x { x = bounds_min_x; }
if x > bounds_max_x - w { x = bounds_max_x - w; }
