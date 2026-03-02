use spatial_codec_draco::EncodeConfig;

#[test]
fn default_config_is_valid() {
    EncodeConfig::default().validate().unwrap();
}

#[test]
fn invalid_quantization_is_rejected() {
    let cfg = EncodeConfig {
        position_quantization_bits: 0,
        ..EncodeConfig::default()
    };
    assert!(cfg.validate().is_err());
}
