use spirv_std::glam::{vec2, vec4, Vec2, Vec4};
use spirv_std::{spirv, Image, Sampler};

#[spirv(vertex)]
pub fn image_vs(
    position: Vec2,
    tex_coord: &mut Vec2,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
) {
    *out_pos = vec4(position.x, position.y, 0.0, 1.0);
    *tex_coord = position + vec2(0.5, 0.5);
}

#[spirv(fragment)]
pub fn image_fs(
    tex_coord: Vec2,
    #[spirv(descriptor_set = 0, binding = 0)] sampler: &Sampler,
    #[spirv(descriptor_set = 0, binding = 1)] image: &Image!(2D, type=f32, sampled),
    f_color: &mut Vec4,
) {
    *f_color = image.sample(*sampler, tex_coord);
}
