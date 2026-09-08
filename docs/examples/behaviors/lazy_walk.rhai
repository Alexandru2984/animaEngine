// Walk right at a configurable speed, wrapping at the desktop edge.
//
//   [characters.behavior]
//   type = "script"
//   path = "behaviors/lazy_walk.rhai"
//
//   [characters.behavior.params]
//   speed = 60.0

x += params.speed * dt;

if x > bounds_max_x - w {
    x = bounds_min_x;
}
