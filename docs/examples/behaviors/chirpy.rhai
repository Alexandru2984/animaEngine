// Chirp on arrival at each end of a patrol.
//
// Shows `play()`: the sound is panned by where the character is, so the
// chirp comes from the left when the mascot is on the left. Put a short
// WAV or OGG in your asset library and point `sound` at it.
//
//   [characters.behavior.params]
//   speed = 90.0
//
// The sound file is named in the script rather than in params because
// params are numbers only.

if !state.contains("dir") {
    state.dir = 1.0;
}

x += state.dir * params.speed * dt;

if x < bounds_min_x {
    x = bounds_min_x;
    state.dir = 1.0;
    play("chirp.ogg");
} else if x > bounds_max_x - w {
    x = bounds_max_x - w;
    state.dir = -1.0;
    play("chirp.ogg");
}
