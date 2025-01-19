pub fn smoothstep(x: f32) -> f32 {
    if x < 0. {
        0.
    } else if x > 1. {
        1.
    } else {
        x * x * (3. - 2. * x)
    }
}
