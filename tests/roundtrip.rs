use spatial_codec_draco::{decode_draco, encode_draco, PointCloudEncodingMethod};

#[test]
fn encode_decode_roundtrip_smoke() {
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

    let encoded = encode_draco(&coords, &colors, PointCloudEncodingMethod::KdTree)
        .expect("encode");
    let (coords2, colors2) = decode_draco(&encoded).expect("decode");

    assert_eq!(coords2.len(), coords.len());
    assert_eq!(colors2.len(), colors.len());
    assert_eq!(coords2.len() % 3, 0);
    assert_eq!(colors2.len() % 3, 0);
}
