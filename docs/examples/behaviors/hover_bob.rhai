// Bob up and down around wherever the character was placed.
//
// Shows two things: accumulating across frames through `state` (a
// top-level `let` would not survive, because Rhai unwinds the scope when
// a run ends), and honouring the reduced-motion preference — this is
// decoration, so it holds still when the user has asked for less motion.
//
//   [characters.behavior.params]
//   amplitude = 20.0
//   period = 2.0

if reduced_motion {
    // Nothing to do; leaving x/y untouched keeps the character still.
} else {
    if !state.contains("rest_y") {
        state.rest_y = y;
        state.phase = 0.0;
    }

    // Guard the period: a hand-edited 0 would divide by zero.
    let period = if params.period > 0.05 { params.period } else { 0.05 };
    state.phase = (state.phase + dt) % period;

    let turns = state.phase / period * 2.0 * PI();
    y = state.rest_y + turns.sin() * params.amplitude;
}
