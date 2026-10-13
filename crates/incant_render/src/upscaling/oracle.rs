//! Independent double-precision references for numerical GPU acceptance. Not a
//! runtime/shader substitute. Coordinates are physical and internal pixel centers.
pub(super) fn source(
    p: [f64; 2],
    output: [f64; 4],
    source: [f64; 4],
    jitter: [f64; 2],
) -> [f64; 2] {
    std::array::from_fn(|i| {
        source[i] + ((p[i] - output[i]) / output[i + 2] + jitter[i]) * source[i + 2]
    })
}
pub(super) fn previous(
    p: [f64; 2],
    output: [f64; 4],
    current_jitter: [f64; 2],
    previous_jitter: [f64; 2],
    motion: [f64; 2],
) -> Option<[f64; 2]> {
    let q = std::array::from_fn(|i| {
        p[i] + (current_jitter[i] + motion[i] - previous_jitter[i]) * output[i + 2]
    });
    (q.iter().all(|x| x.is_finite())
        && (0..2).all(|i| q[i] >= output[i] && q[i] < output[i] + output[i + 2]))
    .then_some(q)
}
pub(super) fn taps(p: [f64; 2], rect: [f64; 4]) -> Option<[([i32; 2], f64); 4]> {
    if !p.iter().all(|x| x.is_finite()) {
        return None;
    }
    let base = p.map(|v| (v - 0.5).floor() as i32);
    let f = std::array::from_fn::<_, 2, _>(|i| p[i] - 0.5 - f64::from(base[i]));
    let result = std::array::from_fn(|n| {
        let x = n % 2;
        let y = n / 2;
        (
            [base[0] + x as i32, base[1] + y as i32],
            (if x == 0 { 1. - f[0] } else { f[0] }) * (if y == 0 { 1. - f[1] } else { f[1] }),
        )
    });
    result
        .iter()
        .all(|(p, w)| {
            *w == 0.
                || (0..2).all(|i| {
                    f64::from(p[i]) + 0.5 >= rect[i]
                        && f64::from(p[i]) + 0.5 < rect[i] + rect[i + 2]
                })
        })
        .then_some(result)
}
pub(super) fn color(v: [f64; 4]) -> [f64; 4] {
    if !v.iter().all(|v| v.is_finite()) || v[3] <= 0. {
        return [0.; 4];
    }
    std::array::from_fn(|i| v[i].clamp(0., if i == 3 { 1. } else { 65504. }))
}
pub(super) fn over(overlay: [f64; 4], history: [f64; 4]) -> [f64; 4] {
    let a = color(overlay);
    let b = color(history);
    color(std::array::from_fn(|i| a[i] + b[i] * (1. - a[3])))
}
pub(super) fn matches(a: f64, b: f64, relative: f64, absolute: f64) -> bool {
    a.is_finite()
        && b.is_finite()
        && a > 0.
        && b > 0.
        && (a - b).abs() <= absolute.max(relative * b)
}
