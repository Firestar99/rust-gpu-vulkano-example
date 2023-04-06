use spirv_std::{Image, spirv};
use spirv_std::glam::{vec2, Vec2, Vec4, vec4};
use spirv_std::image::SampledImage;

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
	#[spirv(descriptor_set = 0, binding = 0)] image: &SampledImage<Image!(2D, type=f32, sampled)>,
	f_color: &mut Vec4,
) {
	// IGNORE THIS UNSAFE
	// SampledImage's sample functions being marked as unsafe are accidental
	// leftovers that will be removed in some upcoming release, see PR
	// https://github.com/EmbarkStudios/rust-gpu/pull/1029
	*f_color = unsafe {
		image.sample(tex_coord)
	};
}
