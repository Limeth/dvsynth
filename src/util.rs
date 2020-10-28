/// https://www.desmos.com/calculator/hmhxxjxnld
pub fn softmax(min: f32, sharpness: f32, x: f32) -> f32 {
    let min = min as f64;
    let sharpness = sharpness as f64;
    let x = x as f64;
    let result = ((1.0 + (sharpness * (x - min)).exp()).ln() / sharpness) + min;

    result as f32
}

/// Do not google images for this function (or do at your own risk)
/// https://www.desmos.com/calculator/miwhjandre
///
/// `softness` describes the radius around the origin in which the result is smooth
fn softabs(softness: f32, x: f32) -> f32 {
    let abs_x = x.abs();

    if abs_x < softness {
        ((x / softness).powi(2) + 1.0) * 0.5 * softness
    } else {
        abs_x
    }
}

/// Do not google images for this function (or do at your own risk)
/// https://www.desmos.com/calculator/miwhjandre
///
/// Variant of `softabs` where f(0) = 0
///
/// https://www.desmos.com/calculator/dxybnuifuw
pub fn softabs2(softness: f32, x: f32) -> f32 {
    let abs_x = x.abs();

    if abs_x < softness {
        (x / softness).powi(2) * 0.5 * softness
    } else {
        abs_x - 0.5 * softness
    }
}

/// A combination of softabs2 and softmax to limit the maximum value
/// https://www.desmos.com/calculator/1j5pkbmxd8
pub fn softminabs(abs_softness: f32, max_sharpness: f32, max: f32, x: f32) -> f32 {
    softmax(-max, max_sharpness, 0.0) - softmax(-max, max_sharpness, -softabs2(abs_softness, x))
}
