#include "wrapper.h"

#include <cstdlib>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

#include <draco/attributes/geometry_attribute.h>
#include <draco/compression/decode.h>
#include <draco/compression/encode.h>
#include <draco/core/decoder_buffer.h>
#include <draco/core/encoder_buffer.h>
#include <draco/point_cloud/point_cloud.h>
#include <draco/point_cloud/point_cloud_builder.h>

namespace {

struct ValidatedConfig {
    uint32_t pos_q;
    uint32_t col_q;
    uint8_t enc_speed;
    uint8_t dec_speed;
};

static DracoWrapperEncodeResult *alloc_encode_result() {
    auto *r = new DracoWrapperEncodeResult();
    r->success = false;
    r->size = 0;
    r->data = nullptr;
    r->error_msg = nullptr;
    return r;
}

static DracoWrapperDecodeResult *alloc_decode_result() {
    auto *r = new DracoWrapperDecodeResult();
    r->success = false;
    r->num_points = 0;
    r->coords = nullptr;
    r->colors = nullptr;
    r->error_msg = nullptr;
    return r;
}

static bool validate_config(const DracoWrapperEncodeConfig *cfg, ValidatedConfig &out, std::string &err) {
    // Defaults chosen to match previous behavior.
    const DracoWrapperEncodeConfig defaults{11u, 8u, 5u, 5u};
    const DracoWrapperEncodeConfig effective = (cfg != nullptr) ? *cfg : defaults;

    auto in_range_u32 = [](uint32_t v, uint32_t lo, uint32_t hi) { return v >= lo && v <= hi; };
    auto in_range_u8 = [](uint8_t v, uint8_t lo, uint8_t hi) { return v >= lo && v <= hi; };

    if (!in_range_u32(effective.position_quantization_bits, 1u, 31u)) {
        err = "position_quantization_bits must be in [1, 31]";
        return false;
    }
    if (!in_range_u32(effective.color_quantization_bits, 1u, 31u)) {
        err = "color_quantization_bits must be in [1, 31]";
        return false;
    }
    if (!in_range_u8(effective.encoding_speed, 0u, 10u)) {
        err = "encoding_speed must be in [0, 10]";
        return false;
    }
    if (!in_range_u8(effective.decoding_speed, 0u, 10u)) {
        err = "decoding_speed must be in [0, 10]";
        return false;
    }

    out.pos_q = effective.position_quantization_bits;
    out.col_q = effective.color_quantization_bits;
    out.enc_speed = effective.encoding_speed;
    out.dec_speed = effective.decoding_speed;
    return true;
}

static char *dup_cstr(const char *s) {
    if (s == nullptr) {
        return nullptr;
    }
    const size_t n = std::strlen(s);
    char *out = static_cast<char *>(std::malloc(n + 1));
    if (out == nullptr) {
        return nullptr;
    }
    std::memcpy(out, s, n);
    out[n] = '\0';
    return out;
}

static char *dup_str(const std::string &s) {
    return dup_cstr(s.c_str());
}

static draco::PointCloudEncodingMethod to_draco_method(DracoWrapperPcEncodingMethod m) {
    switch (m) {
        case DRACO_WRAPPER_POINT_CLOUD_SEQUENTIAL_ENCODING:
            return draco::POINT_CLOUD_SEQUENTIAL_ENCODING;
        case DRACO_WRAPPER_POINT_CLOUD_KD_TREE_ENCODING:
            return draco::POINT_CLOUD_KD_TREE_ENCODING;
        default:
            // Keep this deterministic: default to sequential if unknown.
            return draco::POINT_CLOUD_SEQUENTIAL_ENCODING;
    }
}

} // namespace

extern "C" {

DracoWrapperEncodeResult *draco_wrapper_encode_points_to_draco(
    const float *coords,
    const uint8_t *colors,
    size_t num_points,
    DracoWrapperPcEncodingMethod encoding_method,
    const DracoWrapperEncodeConfig *config) {

    DracoWrapperEncodeResult *result = alloc_encode_result();

    if (coords == nullptr || colors == nullptr) {
        result->error_msg = dup_cstr("Invalid input: coords/colors pointers are null.");
        return result;
    }
    if (num_points == 0) {
        result->error_msg = dup_cstr("Invalid input: num_points must be > 0.");
        return result;
    }

    ValidatedConfig cfg{};
    std::string cfg_err;
    if (!validate_config(config, cfg, cfg_err)) {
        result->error_msg = dup_str(cfg_err);
        return result;
    }

    try {
        draco::PointCloud point_cloud;
        point_cloud.set_num_points(static_cast<uint32_t>(num_points));

        auto position_attribute = std::make_unique<draco::PointAttribute>();
        auto color_attribute = std::make_unique<draco::PointAttribute>();

        position_attribute->Init(
            draco::GeometryAttribute::POSITION,
            3,
            draco::DataType::DT_FLOAT32,
            false,
            point_cloud.num_points());

        color_attribute->Init(
            draco::GeometryAttribute::COLOR,
            3,
            draco::DataType::DT_UINT8,
            true,
            point_cloud.num_points());

        for (uint32_t i = 0; i < point_cloud.num_points(); i++) {
            const size_t base = static_cast<size_t>(i) * 3u;
            position_attribute->SetAttributeValue(
                draco::AttributeValueIndex(i), coords + base);
            color_attribute->SetAttributeValue(
                draco::AttributeValueIndex(i), colors + base);
        }

        (void)point_cloud.AddAttribute(std::move(position_attribute));
        (void)point_cloud.AddAttribute(std::move(color_attribute));

        draco::Encoder encoder;
        draco::EncoderBuffer encoder_buffer;

        encoder.SetEncodingMethod(to_draco_method(encoding_method));
        encoder.SetAttributeQuantization(draco::GeometryAttribute::POSITION, cfg.pos_q);
        encoder.SetAttributeQuantization(draco::GeometryAttribute::COLOR, cfg.col_q);
        encoder.SetSpeedOptions(cfg.enc_speed, cfg.dec_speed);

        draco::Status status = encoder.EncodePointCloudToBuffer(point_cloud, &encoder_buffer);
        if (!status.ok()) {
            throw std::runtime_error(std::string("Failed to encode point cloud: ") + status.error_msg());
        }

        uint8_t *encoded_data = new uint8_t[encoder_buffer.size()];
        std::memcpy(encoded_data, encoder_buffer.data(), encoder_buffer.size());

        result->success = true;
        result->data = encoded_data;
        result->size = encoder_buffer.size();
        return result;

    } catch (const std::exception &e) {
        result->error_msg = dup_cstr(e.what());
        return result;
    } catch (...) {
        result->error_msg = dup_cstr("Unknown error occurred during encoding.");
        return result;
    }
}

DracoWrapperDecodeResult *draco_wrapper_decode_draco_data(const uint8_t *encoded_data, size_t encoded_size) {
    DracoWrapperDecodeResult *result = alloc_decode_result();

    if (encoded_data == nullptr) {
        result->error_msg = dup_cstr("Invalid input: encoded_data pointer is null.");
        return result;
    }
    if (encoded_size == 0) {
        result->error_msg = dup_cstr("Invalid input: encoded_size must be > 0.");
        return result;
    }

    try {
        draco::PointCloud point_cloud;
        draco::DecoderBuffer decoder_buffer;
        decoder_buffer.Init(reinterpret_cast<const char *>(encoded_data), encoded_size);

        draco::Decoder decoder;
        draco::Status status = decoder.DecodeBufferToGeometry(&decoder_buffer, &point_cloud);
        if (!status.ok()) {
            throw std::runtime_error(std::string("Failed to decode point cloud: ") + status.error_msg());
        }

        const size_t num_points = point_cloud.num_points();
        result->num_points = num_points;

        int pos_att_id = point_cloud.GetNamedAttributeId(draco::GeometryAttribute::POSITION);
        if (pos_att_id < 0) {
            throw std::runtime_error("Position attribute not found");
        }
        const draco::PointAttribute *pos_att = point_cloud.GetAttributeByUniqueId(pos_att_id);

        int col_att_id = point_cloud.GetNamedAttributeId(draco::GeometryAttribute::COLOR);
        if (col_att_id < 0) {
            throw std::runtime_error("Color attribute not found");
        }
        const draco::PointAttribute *col_att = point_cloud.GetAttributeByUniqueId(col_att_id);

        result->coords = new float[num_points * 3];
        result->colors = new uint8_t[num_points * 3];

        for (draco::PointIndex i(0); i < point_cloud.num_points(); ++i) {
            const size_t base = static_cast<size_t>(i.value()) * 3u;
            pos_att->GetValue(draco::AttributeValueIndex(i.value()), result->coords + base);
            col_att->GetValue(draco::AttributeValueIndex(i.value()), result->colors + base);
        }

        result->success = true;
        return result;

    } catch (const std::exception &e) {
        result->error_msg = dup_cstr(e.what());
        return result;
    } catch (...) {
        result->error_msg = dup_cstr("Unknown error occurred during decoding.");
        return result;
    }
}

void draco_wrapper_free_encode_result(DracoWrapperEncodeResult *result) {
    if (result == nullptr) {
        return;
    }
    if (result->data != nullptr) {
        delete[] result->data;
        result->data = nullptr;
    }
    if (result->error_msg != nullptr) {
        std::free(result->error_msg);
        result->error_msg = nullptr;
    }
    delete result;
}

void draco_wrapper_free_decode_result(DracoWrapperDecodeResult *result) {
    if (result == nullptr) {
        return;
    }
    if (result->coords != nullptr) {
        delete[] result->coords;
        result->coords = nullptr;
    }
    if (result->colors != nullptr) {
        delete[] result->colors;
        result->colors = nullptr;
    }
    if (result->error_msg != nullptr) {
        std::free(result->error_msg);
        result->error_msg = nullptr;
    }
    delete result;
}

} // extern "C"
