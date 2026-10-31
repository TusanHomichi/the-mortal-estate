//! Models the kits don't have, built from simple solids and written as
//! glTF: low and faceted, to sit beside the kits' own. Each is a function of
//! nothing, so its file only changes when its code does, and a test holds
//! the committed file to what the code makes.

mod gltf;

pub use gltf::Part;

/// Every generated model, by its file name under `assets/models/generated/`.
pub fn all() -> Vec<(&'static str, String)> {
    vec![("dun.gltf", dun())]
}

/// The dun: low and broad, its head carried forward and down, a ridge of
/// rust-coloured bristles along its back, and two pale eyes. About 1.6 m long,
/// 0.8 m wide and 1.1 m high to the tips of its bristles; its nose points +Z,
/// as glTF faces.
pub fn dun() -> String {
    let mut hide = Part::new("hide", [0.23, 0.2, 0.18], 0.9, [0.0; 3]);
    let mut bristles = Part::new("bristles", [0.42, 0.2, 0.11], 0.8, [0.0; 3]);
    let mut eyes = Part::new("eyes", [0.95, 0.9, 0.7], 0.4, [1.0, 0.85, 0.5]);

    // The body, an egg lower at the back; the head, forward and low.
    hide.ellipsoid([0.0, 0.45, -0.05], [0.42, 0.3, 0.6], 9, 5);
    hide.ellipsoid([0.0, 0.4, 0.58], [0.24, 0.2, 0.28], 7, 4);
    // A snout.
    hide.ellipsoid([0.0, 0.33, 0.8], [0.13, 0.1, 0.14], 6, 3);
    // Four short legs, splayed.
    for (x, z) in [(-0.3, 0.32), (0.3, 0.32), (-0.3, -0.42), (0.3, -0.42)] {
        hide.post([x, 0.0, z], [x * 0.85, 0.36, z * 0.9], 0.08, 5);
    }
    // The ridge: bristles from the crown to the haunches, leaning back,
    // longest over the shoulders.
    for i in 0..11 {
        let t = i as f32 / 10.0;
        let z = 0.62 - t * 1.15;
        let along = (z + 0.05) / 0.6;
        let top = 0.45 + 0.3 * (1.0 - along * along).max(0.0).sqrt();
        let length = 0.18 + 0.2 * (1.0 - (t - 0.3).abs() * 1.6).max(0.0);
        for x in [-0.07, 0.07] {
            let x = if i % 2 == 0 { x } else { x * 1.6 };
            bristles.spike(
                [x, top - 0.04, z],
                [x * 1.8, top + length, z - length * 0.7],
                0.05,
                4,
            );
        }
    }
    for x in [-0.11, 0.11] {
        eyes.ellipsoid([x, 0.47, 0.78], [0.035, 0.035, 0.03], 5, 3);
    }
    gltf::write(&[hide, bristles, eyes])
}
