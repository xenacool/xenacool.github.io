//! Deterministic, renderer-independent preprocessing for spritestack slices.
//!
//! This module deliberately works on raw RGBA bytes so it can be shared by
//! build-time and runtime asset ingress without coupling either side to a
//! particular image crate or WebGL API.

use std::collections::VecDeque;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpritestackProcessConfig {
    pub alpha_cutoff: u8,
    pub fill_holes: bool,
}

impl Default for SpritestackProcessConfig {
    fn default() -> Self {
        Self {
            alpha_cutoff: 128,
            fill_holes: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpritestackProcessStats {
    pub holes_filled: usize,
    pub alpha_pixels_changed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpritestackProcessError {
    ZeroSizedImage,
    InvalidBufferLength { expected: usize, actual: usize },
    DimensionsOverflow,
}

impl fmt::Display for SpritestackProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSizedImage => write!(f, "spritestack slice has zero width or height"),
            Self::InvalidBufferLength { expected, actual } => write!(
                f,
                "spritestack RGBA buffer has {actual} bytes; expected {expected}"
            ),
            Self::DimensionsOverflow => write!(f, "spritestack dimensions overflow buffer size"),
        }
    }
}

impl std::error::Error for SpritestackProcessError {}

/// Normalize alpha and fill transparent cavities in one deterministic pass.
pub fn process_slice(
    width: usize,
    height: usize,
    rgba: &mut [u8],
    config: SpritestackProcessConfig,
) -> Result<SpritestackProcessStats, SpritestackProcessError> {
    let pixels = width
        .checked_mul(height)
        .ok_or(SpritestackProcessError::DimensionsOverflow)?;
    if width == 0 || height == 0 {
        return Err(SpritestackProcessError::ZeroSizedImage);
    }
    let expected = pixels
        .checked_mul(4)
        .ok_or(SpritestackProcessError::DimensionsOverflow)?;
    if rgba.len() != expected {
        return Err(SpritestackProcessError::InvalidBufferLength {
            expected,
            actual: rgba.len(),
        });
    }

    let mut stats = SpritestackProcessStats::default();
    for pixel in rgba.chunks_exact_mut(4) {
        let normalized = if pixel[3] >= config.alpha_cutoff {
            255
        } else {
            0
        };
        if pixel[3] != normalized {
            stats.alpha_pixels_changed += 1;
            pixel[3] = normalized;
        }
    }

    if config.fill_holes {
        stats.holes_filled = fill_enclosed_transparency(width, height, rgba);
    }
    Ok(stats)
}

fn fill_enclosed_transparency(width: usize, height: usize, rgba: &mut [u8]) -> usize {
    let mut exterior = vec![false; width * height];
    let mut queue = VecDeque::new();

    for x in 0..width {
        enqueue_exterior(x, 0, width, rgba, &mut exterior, &mut queue);
        enqueue_exterior(x, height - 1, width, rgba, &mut exterior, &mut queue);
    }
    for y in 0..height {
        enqueue_exterior(0, y, width, rgba, &mut exterior, &mut queue);
        enqueue_exterior(width - 1, y, width, rgba, &mut exterior, &mut queue);
    }

    while let Some(index) = queue.pop_front() {
        let x = index % width;
        let y = index / width;
        for (nx, ny) in neighbors(x, y, width, height) {
            let neighbor = ny * width + nx;
            if !exterior[neighbor] && alpha_at(rgba, neighbor) == 0 {
                exterior[neighbor] = true;
                queue.push_back(neighbor);
            }
        }
    }

    let mut filled = 0;
    for index in 0..width * height {
        if alpha_at(rgba, index) == 0 && !exterior[index] {
            let color = nearest_opaque_color(index, width, height, rgba);
            let pixel = &mut rgba[index * 4..index * 4 + 4];
            pixel[..3].copy_from_slice(&color);
            pixel[3] = 255;
            filled += 1;
        }
    }
    filled
}

fn enqueue_exterior(
    x: usize,
    y: usize,
    width: usize,
    rgba: &[u8],
    exterior: &mut [bool],
    queue: &mut VecDeque<usize>,
) {
    let index = y * width + x;
    if !exterior[index] && alpha_at(rgba, index) == 0 {
        exterior[index] = true;
        queue.push_back(index);
    }
}

fn nearest_opaque_color(index: usize, width: usize, height: usize, rgba: &[u8]) -> [u8; 3] {
    let mut visited = vec![false; width * height];
    let mut queue = VecDeque::from([index]);
    visited[index] = true;

    while let Some(current) = queue.pop_front() {
        if alpha_at(rgba, current) != 0 {
            let start = current * 4;
            return [rgba[start], rgba[start + 1], rgba[start + 2]];
        }
        let x = current % width;
        let y = current / width;
        for (nx, ny) in neighbors(x, y, width, height) {
            let neighbor = ny * width + nx;
            if !visited[neighbor] {
                visited[neighbor] = true;
                queue.push_back(neighbor);
            }
        }
    }
    [0, 0, 0]
}

fn neighbors(x: usize, y: usize, width: usize, height: usize) -> [(usize, usize); 4] {
    [
        (x.saturating_sub(1), y),
        ((x + 1).min(width - 1), y),
        (x, y.saturating_sub(1)),
        (x, (y + 1).min(height - 1)),
    ]
}

fn alpha_at(rgba: &[u8], index: usize) -> u8 {
    rgba[index * 4 + 3]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: usize, height: usize, pixels: &[[u8; 4]]) -> Vec<u8> {
        assert_eq!(pixels.len(), width * height);
        pixels.iter().flat_map(|pixel| *pixel).collect()
    }

    #[test]
    fn fills_cavity_with_deterministic_nearest_color() {
        let mut rgba = image(
            3,
            3,
            &[
                [10, 0, 0, 255],
                [10, 0, 0, 255],
                [10, 0, 0, 255],
                [10, 0, 0, 255],
                [0, 0, 0, 0],
                [20, 0, 0, 255],
                [20, 0, 0, 255],
                [20, 0, 0, 255],
                [20, 0, 0, 255],
            ],
        );
        let stats = process_slice(3, 3, &mut rgba, Default::default()).unwrap();
        assert_eq!(stats.holes_filled, 1);
        assert_eq!(&rgba[16..20], &[10, 0, 0, 255]);
    }

    #[test]
    fn preserves_border_connected_transparency() {
        let mut rgba = image(
            3,
            3,
            &[
                [0, 0, 0, 0],
                [1, 2, 3, 255],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [1, 2, 3, 255],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [1, 2, 3, 255],
                [0, 0, 0, 0],
            ],
        );
        let stats = process_slice(3, 3, &mut rgba, Default::default()).unwrap();
        assert_eq!(stats.holes_filled, 0);
        assert_eq!(rgba[7], 255);
        assert_eq!(rgba[0], 0);
    }

    #[test]
    fn normalizes_alpha_at_cutoff_and_is_idempotent() {
        let mut rgba = image(2, 1, &[[1, 2, 3, 127], [4, 5, 6, 128]]);
        let config = SpritestackProcessConfig::default();
        let first = process_slice(2, 1, &mut rgba, config).unwrap();
        let expected = rgba.clone();
        let second = process_slice(2, 1, &mut rgba, config).unwrap();
        assert_eq!(first.alpha_pixels_changed, 2);
        assert_eq!(second.alpha_pixels_changed, 0);
        assert_eq!(rgba, expected);
    }

    #[test]
    fn rejects_invalid_dimensions_and_buffers() {
        let mut rgba = vec![];
        assert_eq!(
            process_slice(0, 1, &mut rgba, Default::default()),
            Err(SpritestackProcessError::ZeroSizedImage)
        );
        assert_eq!(
            process_slice(1, 1, &mut rgba, Default::default()),
            Err(SpritestackProcessError::InvalidBufferLength {
                expected: 4,
                actual: 0
            })
        );
    }

    #[test]
    fn can_disable_hole_filling() {
        let mut rgba = image(
            3,
            3,
            &[
                [1, 1, 1, 255],
                [1, 1, 1, 255],
                [1, 1, 1, 255],
                [1, 1, 1, 255],
                [0, 0, 0, 0],
                [1, 1, 1, 255],
                [1, 1, 1, 255],
                [1, 1, 1, 255],
                [1, 1, 1, 255],
            ],
        );
        let config = SpritestackProcessConfig {
            fill_holes: false,
            ..Default::default()
        };
        let stats = process_slice(3, 3, &mut rgba, config).unwrap();
        assert_eq!(stats.holes_filled, 0);
        assert_eq!(rgba[19], 0);
    }
}
