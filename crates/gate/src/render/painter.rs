use pystral_core::domain::PainterCommand;
use tiny_skia::{Pixmap, Paint, Stroke, PathBuilder, FillRule, Transform};
use web_sys::{WebGlRenderingContext as GL, WebGlTexture};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewSide {
    Front,
    Mirrored,
}

pub fn render_commands_to_texture(gl: &GL, commands: &[PainterCommand], width: u32, height: u32, side: ViewSide) -> Option<WebGlTexture> {
    if commands.is_empty() {
        return None;
    }

    let mut pixmap = Pixmap::new(width, height)?;
    let mut paint = Paint { anti_alias: true, ..Default::default() };
    let mut stroke = Stroke::default();
    
    // Group commands into draw calls (ending in Fill or Stroke)
    let mut draw_calls = Vec::new();
    let mut current_call = Vec::new();
    for cmd in commands {
        current_call.push(cmd.clone());
        if matches!(cmd, PainterCommand::Fill | PainterCommand::Stroke) {
            draw_calls.push(std::mem::take(&mut current_call));
        }
    }
    if !current_call.is_empty() {
        draw_calls.push(current_call);
    }

    if side == ViewSide::Mirrored {
        draw_calls.reverse();
    }

    for call in draw_calls {
        let mut path_builder = PathBuilder::new();
        for cmd in call {
            match cmd {
                PainterCommand::MoveTo(x, y, _z) => {
                    let rx = if side == ViewSide::Mirrored { width as f32 - x } else { x };
                    path_builder.move_to(rx, y);
                }
                PainterCommand::LineTo(x, y, _z) => {
                    let rx = if side == ViewSide::Mirrored { width as f32 - x } else { x };
                    path_builder.line_to(rx, y);
                }
                PainterCommand::QuadTo(x1, y1, _z1, x, y, _z) => {
                    let rx1 = if side == ViewSide::Mirrored { width as f32 - x1 } else { x1 };
                    let rx = if side == ViewSide::Mirrored { width as f32 - x } else { x };
                    path_builder.quad_to(rx1, y1, rx, y);
                }
                PainterCommand::CubicTo(x1, y1, _z1, x2, y2, _z2, x, y, _z) => {
                    let rx1 = if side == ViewSide::Mirrored { width as f32 - x1 } else { x1 };
                    let rx2 = if side == ViewSide::Mirrored { width as f32 - x2 } else { x2 };
                    let rx = if side == ViewSide::Mirrored { width as f32 - x } else { x };
                    path_builder.cubic_to(rx1, y1, rx2, y2, rx, y);
                }
                PainterCommand::Close => path_builder.close(),
                PainterCommand::SetColor(front, mirrored) => {
                    let rgba = if side == ViewSide::Front { front } else { mirrored };
                    paint.set_color_rgba8(
                        (rgba[0] * 255.0) as u8,
                        (rgba[1] * 255.0) as u8,
                        (rgba[2] * 255.0) as u8,
                        (rgba[3] * 255.0) as u8,
                    );
                }
                PainterCommand::SetStrokeWidth(w) => {
                    stroke.width = w;
                }
                PainterCommand::Fill => {
                    if let Some(path) = path_builder.finish() {
                        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                    }
                    path_builder = PathBuilder::new();
                }
                PainterCommand::Stroke => {
                    if let Some(path) = path_builder.finish() {
                        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                    }
                    path_builder = PathBuilder::new();
                }
            }
        }
    }

    let texture = gl.create_texture()?;
    gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
    
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        GL::TEXTURE_2D,
        0,
        GL::RGBA as i32,
        width as i32,
        height as i32,
        0,
        GL::RGBA,
        GL::UNSIGNED_BYTE,
        Some(pixmap.data()),
    ).ok()?;

    #[allow(clippy::cast_possible_wrap)]
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
    #[allow(clippy::cast_possible_wrap)]
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);

    Some(texture)
}
