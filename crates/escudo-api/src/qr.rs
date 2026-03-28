use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::Luma;
use qrcode::QrCode;

pub fn generate_qr_base64(data: &str) -> anyhow::Result<String> {
    let code = QrCode::new(data.as_bytes())?;
    let image = code.render::<Luma<u8>>().quiet_zone(true).build();

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::L8,
    )?;

    Ok(BASE64.encode(&png_bytes))
}
