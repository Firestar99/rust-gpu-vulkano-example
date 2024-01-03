#[cfg(not(feature = "use-glsl-shader"))]
pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        root_path_env: "SHADER_OUT_DIR",
        bytes: "image_shader-image_vs.spv",
    }

    pub const ENTRY_POINT: &str = "image_shader::image_vs";
}

#[cfg(not(feature = "use-glsl-shader"))]
pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        root_path_env: "SHADER_OUT_DIR",
        bytes: "image_shader-image_fs.spv",
    }

    pub const ENTRY_POINT: &str = "image_shader::image_fs";
}

#[cfg(feature = "use-glsl-shader")]
pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) in vec2 position;
            layout(location = 0) out vec2 tex_coords;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
                tex_coords = position + vec2(0.5);
            }
        ",
    }

    pub const ENTRY_POINT: &str = "main";
}

#[cfg(feature = "use-glsl-shader")]
pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) in vec2 tex_coords;
            layout(location = 0) out vec4 f_color;

            layout(set = 0, binding = 0) uniform sampler2D tex;

            void main() {
                f_color = texture(tex, tex_coords);
            }
        ",
    }

    pub const ENTRY_POINT: &str = "main";
}
