// Pace faster when the machine is working hard, doze when it isn't.
//
// `cpu` and `mem` are fractions from 0.0 to 1.0, and they are *aggregate*
// only — there is deliberately no way for a script to learn what is
// running, just that the machine is busy. See src/sysload.rs.
//
//   [characters.behavior.params]
//   idle_speed = 20.0
//   busy_speed = 160.0

// Ease toward the target speed instead of snapping, so a brief CPU spike
// doesn't make the character twitch.
if !state.contains("speed") {
    state.speed = params.idle_speed;
}

let target = params.idle_speed + (params.busy_speed - params.idle_speed) * cpu;
state.speed += (target - state.speed) * 2.0 * dt;

if !state.contains("dir") {
    state.dir = 1.0;
}

x += state.dir * state.speed * dt;

if x < bounds_min_x {
    x = bounds_min_x;
    state.dir = 1.0;
} else if x > bounds_max_x - w {
    x = bounds_max_x - w;
    state.dir = -1.0;
}
