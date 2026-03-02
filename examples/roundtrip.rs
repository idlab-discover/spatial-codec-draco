//! Minimal encode → decode roundtrip.

use spatial_codec_draco::{decode_draco, encode_draco, PointCloudEncodingMethod};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let coords: Vec<f32> = vec![
        0.0, 0.0, 0.0,
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
    ];
    let colors: Vec<u8> = vec![
        255, 0, 0,
        0, 255, 0,
        0, 0, 255,
    ];

    let encoded = encode_draco(&coords, &colors, PointCloudEncodingMethod::KdTree)?;
    let (coords2, colors2) = decode_draco(&encoded)?;

    println!("Encoded {} bytes", encoded.len());
    println!("Decoded {} points", coords2.len() / 3);

    assert_eq!(coords2.len(), coords.len());
    assert_eq!(colors2.len(), colors.len());

    Ok(())
}
