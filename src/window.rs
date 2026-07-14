//! Native graphical window support for Ferrum's painted canvas.

use crate::paint::Canvas;
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowError {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowFrame {
    pub title: String,
    pub canvas: Canvas,
}

impl fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "window error: {}", self.message)
    }
}

impl Error for WindowError {}

pub fn show(title: &str, canvas: &Canvas) -> Result<(), WindowError> {
    show_interactive(title, canvas, |_, _| Ok(None))
}

/// Opens a window and asks the caller for a new frame after each left click.
pub fn show_interactive<F>(title: &str, canvas: &Canvas, mut on_click: F) -> Result<(), WindowError>
where
    F: FnMut(f32, f32) -> Result<Option<WindowFrame>, String>,
{
    let width = canvas.width as usize;
    let height = canvas.height as usize;
    let mut buffer = rgb_buffer(canvas);
    let mut window = Window::new(title, width, height, WindowOptions::default()).map_err(error)?;
    window.set_target_fps(60);
    let mut mouse_was_down = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mouse_is_down = window.get_mouse_down(MouseButton::Left);
        if mouse_is_down
            && !mouse_was_down
            && let Some((x, y)) = window.get_mouse_pos(MouseMode::Clamp)
            && let Some(frame) = on_click(x, y).map_err(|message| WindowError { message })?
        {
            if frame.canvas.width as usize != width || frame.canvas.height as usize != height {
                return Err(WindowError {
                    message: "interactive frames must keep the original window size".into(),
                });
            }
            window.set_title(&frame.title);
            buffer = rgb_buffer(&frame.canvas);
        }
        mouse_was_down = mouse_is_down;
        window
            .update_with_buffer(&buffer, width, height)
            .map_err(error)?;
    }
    Ok(())
}

pub fn rgb_buffer(canvas: &Canvas) -> Vec<u32> {
    canvas
        .pixels
        .iter()
        .map(|pixel| {
            (u32::from(pixel.red) << 16) | (u32::from(pixel.green) << 8) | u32::from(pixel.blue)
        })
        .collect()
}

fn error(error: impl fmt::Display) -> WindowError {
    WindowError {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::Color;

    #[test]
    fn converts_canvas_pixels_to_minifb_rgb() {
        let canvas = Canvas::new(
            1,
            1,
            Color {
                red: 0x12,
                green: 0x34,
                blue: 0x56,
                alpha: 255,
            },
        );
        assert_eq!(rgb_buffer(&canvas), [0x0012_3456]);
    }
}
