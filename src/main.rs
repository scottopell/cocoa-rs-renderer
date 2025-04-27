#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case)]

use std::cell::{OnceCell, RefCell};
use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, ProtocolObject};
use objc2::AnyThread;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSAutoresizingMaskOptions,
    NSBackingStoreType, NSBezelStyle, NSBitmapImageRep, NSButton, NSEvent, NSImage, NSImageScaling,
    NSImageView, NSResponder, NSScrollView, NSSlider, NSWindow, NSWindowDelegate,
    NSWindowStyleMask,
};
use objc2_foundation::{
    ns_string, NSArray, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSURL,
};

// Structure to hold source pattern and debug pixel data
#[derive(Debug)]
struct SourcePattern {
    buffer: Vec<u8>,
    width: usize,
    height: usize,
    bytes_per_row: usize,
}

// Structure to hold rendering information
#[derive(Debug)]
struct ImageRenderer {
    // Source image dimensions
    source_width: usize,
    source_height: usize,

    // Current view information
    zoom_level: f64,
    view_x: f64,
    view_y: f64,

    // Pattern type
    pattern_type: PatternType,

    // Source pattern with debug borders
    source_pattern: Option<SourcePattern>,

    // Text content for text pattern
    primary_text: Option<String>,
    secondary_text: Option<String>,
}

// Enum to represent different pattern types
#[derive(Debug)]
enum PatternType {
    Checkerboard,
    Gradient,
    Text,
}

impl ImageRenderer {
    fn new(pattern_type: PatternType, width: usize, height: usize) -> Self {
        let mut renderer = Self {
            source_width: width,
            source_height: height,
            zoom_level: 1.0,
            view_x: 0.0,
            view_y: 0.0,
            pattern_type,
            source_pattern: None,
            primary_text: None,
            secondary_text: None,
        };

        // Create the source pattern
        renderer.generate_source_pattern();

        renderer
    }

    fn set_zoom(&mut self, zoom: f64) {
        self.zoom_level = zoom.max(0.1).min(10.0);
    }

    fn set_pan(&mut self, x: f64, y: f64) {
        self.view_x = x;
        self.view_y = y;
    }

    // Change the pattern type while preserving other settings
    fn change_pattern_type(&mut self, pattern_type: PatternType) {
        self.pattern_type = pattern_type;
        // Regenerate the source pattern with the new type
        self.generate_source_pattern();
    }

    fn set_text(&mut self, primary: Option<String>, secondary: Option<String>) {
        self.primary_text = primary;
        self.secondary_text = secondary;
        // If we're using the text pattern, regenerate it with the new text
        if let PatternType::Text = self.pattern_type {
            self.generate_source_pattern();
        }
    }

    fn get_viewport_size(&self) -> (usize, usize) {
        let width = (self.source_width as f64 * self.zoom_level) as usize;
        let height = (self.source_height as f64 * self.zoom_level) as usize;
        (width, height)
    }

    // Generate the source pattern with borders
    fn generate_source_pattern(&mut self) {
        let width = self.source_width;
        let height = self.source_height;
        let bytes_per_row = width * 4; // RGBA format
        let buffer_size = bytes_per_row * height;
        let mut buffer = vec![0; buffer_size];

        // Generate the base pattern
        match self.pattern_type {
            PatternType::Checkerboard => {
                self.generate_checkerboard(&mut buffer, width, height, bytes_per_row)
            }
            PatternType::Gradient => {
                self.generate_gradient(&mut buffer, width, height, bytes_per_row)
            }
            PatternType::Text => self.generate_text(&mut buffer, width, height, bytes_per_row),
        }

        // Add debug borders and corners
        self.add_debug_borders(&mut buffer, width, height, bytes_per_row);

        // Store the pattern
        self.source_pattern = Some(SourcePattern {
            buffer,
            width,
            height,
            bytes_per_row,
        });
    }

    // Generate a checkerboard pattern
    fn generate_checkerboard(
        &self,
        buffer: &mut Vec<u8>,
        width: usize,
        height: usize,
        bytes_per_row: usize,
    ) {
        let square_size = 20; // Size of each checkerboard square

        for y in 0..height {
            for x in 0..width {
                let idx = y * bytes_per_row + x * 4;

                // Determine if this pixel should be black or white
                let is_white = ((x / square_size) + (y / square_size)) % 2 == 0;

                let color = if is_white { 255u8 } else { 0u8 };

                buffer[idx] = color; // Red
                buffer[idx + 1] = color; // Green
                buffer[idx + 2] = color; // Blue
                buffer[idx + 3] = 255; // Alpha
            }
        }
    }

    // Generate a gradient pattern
    fn generate_gradient(
        &self,
        buffer: &mut Vec<u8>,
        width: usize,
        height: usize,
        bytes_per_row: usize,
    ) {
        for y in 0..height {
            for x in 0..width {
                let idx = y * bytes_per_row + x * 4;

                // Create a blue to white gradient
                let r = ((x as f64) / (width as f64) * 255.0) as u8;
                let g = ((y as f64) / (height as f64) * 255.0) as u8;
                let b = 200u8;

                buffer[idx] = r; // Red
                buffer[idx + 1] = g; // Green
                buffer[idx + 2] = b; // Blue
                buffer[idx + 3] = 255; // Alpha
            }
        }
    }

    // Generate a text pattern with improved rendering
    fn generate_text(
        &self,
        buffer: &mut Vec<u8>,
        width: usize,
        height: usize,
        bytes_per_row: usize,
    ) {
        // First, fill the entire buffer with a light blue-gray background
        for y in 0..height {
            for x in 0..width {
                let idx = y * bytes_per_row + x * 4;
                buffer[idx] = 230; // Red
                buffer[idx + 1] = 235; // Green
                buffer[idx + 2] = 240; // Blue
                buffer[idx + 3] = 255; // Alpha
            }
        }

        // Characters we can draw (basic ASCII representation)
        let characters = [
            // C
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0],
                [0, 1, 1, 1, 0],
            ],
            // O
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
            ],
            // M
            [
                [1, 0, 0, 0, 1],
                [1, 1, 0, 1, 1],
                [1, 0, 1, 0, 1],
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
            ],
            // I
            [
                [0, 1, 1, 1, 0],
                [0, 0, 1, 0, 0],
                [0, 0, 1, 0, 0],
                [0, 0, 1, 0, 0],
                [0, 1, 1, 1, 0],
            ],
            // N
            [
                [1, 0, 0, 0, 1],
                [1, 1, 0, 0, 1],
                [1, 0, 1, 0, 1],
                [1, 0, 0, 1, 1],
                [1, 0, 0, 0, 1],
            ],
            // G
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 0],
                [1, 0, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
            ],
            // S
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 0],
                [0, 1, 1, 1, 0],
                [0, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
            ],
            // P
            [
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0],
            ],
            // J
            [
                [0, 0, 1, 1, 0],
                [0, 0, 0, 1, 0],
                [0, 0, 0, 1, 0],
                [1, 0, 0, 1, 0],
                [0, 1, 1, 0, 0],
            ],
            // 2
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [0, 0, 1, 1, 0],
                [0, 1, 0, 0, 0],
                [1, 1, 1, 1, 1],
            ],
            // SPACE
            [
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
            ],
            // F
            [
                [1, 1, 1, 1, 1],
                [1, 0, 0, 0, 0],
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0],
            ],
            // L
            [
                [1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0],
                [1, 1, 1, 1, 1],
            ],
            // E
            [
                [1, 1, 1, 1, 1],
                [1, 0, 0, 0, 0],
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 0],
                [1, 1, 1, 1, 1],
            ],
            // D
            [
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
                [1, 1, 1, 1, 0],
            ],
            // T
            [
                [1, 1, 1, 1, 1],
                [0, 0, 1, 0, 0],
                [0, 0, 1, 0, 0],
                [0, 0, 1, 0, 0],
                [0, 0, 1, 0, 0],
            ],
            // A
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [1, 1, 1, 1, 1],
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
            ],
            // R
            [
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [1, 1, 1, 1, 0],
                [1, 0, 1, 0, 0],
                [1, 0, 0, 1, 0],
            ],
            // B
            [
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [1, 1, 1, 1, 0],
            ],
            // 0
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
            ],
            // 1
            [
                [0, 0, 1, 0, 0],
                [0, 1, 1, 0, 0],
                [0, 0, 1, 0, 0],
                [0, 0, 1, 0, 0],
                [0, 1, 1, 1, 0],
            ],
            // 3
            [
                [0, 1, 1, 1, 0],
                [0, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
                [0, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
            ],
            // 4
            [
                [1, 0, 0, 0, 1],
                [1, 0, 0, 0, 1],
                [1, 1, 1, 1, 1],
                [0, 0, 0, 0, 1],
                [0, 0, 0, 0, 1],
            ],
            // 5
            [
                [1, 1, 1, 1, 1],
                [1, 0, 0, 0, 0],
                [1, 1, 1, 1, 0],
                [0, 0, 0, 0, 1],
                [1, 1, 1, 1, 0],
            ],
            // 6
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 0],
                [1, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
            ],
            // 7
            [
                [1, 1, 1, 1, 1],
                [0, 0, 0, 0, 1],
                [0, 0, 0, 1, 0],
                [0, 0, 1, 0, 0],
                [0, 1, 0, 0, 0],
            ],
            // 8
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
            ],
            // 9
            [
                [0, 1, 1, 1, 0],
                [1, 0, 0, 0, 1],
                [0, 1, 1, 1, 1],
                [0, 0, 0, 0, 1],
                [0, 1, 1, 1, 0],
            ],
            // - (dash)
            [
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [1, 1, 1, 1, 1],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
            ],
            // . (period)
            [
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 1, 0, 0],
            ],
        ];

        // Map characters to their index
        let char_map: std::collections::HashMap<char, usize> = [
            ('C', 0),
            ('O', 1),
            ('M', 2),
            ('I', 3),
            ('N', 4),
            ('G', 5),
            ('S', 6),
            ('P', 7),
            ('J', 8),
            ('2', 9),
            (' ', 10),
            ('F', 11),
            ('L', 12),
            ('E', 13),
            ('D', 14),
            ('T', 15),
            ('A', 16),
            ('R', 17),
            ('B', 18),
            ('0', 19),
            ('1', 20),
            ('3', 21),
            ('4', 22),
            ('5', 23),
            ('6', 24),
            ('7', 25),
            ('8', 26),
            ('9', 27),
            ('-', 28),
            ('.', 29),
        ]
        .iter()
        .cloned()
        .collect();

        // The primary text to display (default to "COMING SOON")
        let primary = self.primary_text.as_deref().unwrap_or("COMING SOON");

        // Simple sizes and positions
        let char_width = 32;
        let char_height = 40;
        let char_padding = 4;

        // Calculate centered positions
        let text_width = primary.len() * (char_width + char_padding);
        let start_x = (width - text_width) / 2;
        let start_y = height / 2 - char_height;

        // Draw the primary text
        self.draw_text(
            buffer,
            width,
            height,
            bytes_per_row,
            &characters,
            &char_map,
            primary,
            start_x,
            start_y,
            char_width,
            char_height,
            char_padding,
            [30, 30, 180],
        ); // Dark blue color

        // Draw secondary text if available (like filename)
        if let Some(secondary) = &self.secondary_text {
            let secondary_text = secondary;
            let smaller_char_width = 16;
            let smaller_char_height = 20;
            let smaller_padding = 2;

            // Limit the secondary text length if needed
            let display_text = if secondary_text.len() > 30 {
                format!("{}...", &secondary_text[0..27])
            } else {
                secondary_text.to_string()
            };

            let secondary_text_width = display_text.len() * (smaller_char_width + smaller_padding);
            let secondary_x = (width - secondary_text_width) / 2;
            let secondary_y = start_y + char_height + 40; // Below the primary text

            self.draw_text(
                buffer,
                width,
                height,
                bytes_per_row,
                &characters,
                &char_map,
                &display_text.to_uppercase(),
                secondary_x,
                secondary_y,
                smaller_char_width,
                smaller_char_height,
                smaller_padding,
                [20, 120, 20],
            ); // Dark green color
        }

        // Add "FILE SELECTED" text at the bottom if there's a secondary text
        if self.secondary_text.is_some() {
            let info_text = "FILE SELECTED";
            let small_char_width = 12;
            let small_char_height = 15;
            let small_padding = 1;

            let info_text_width = info_text.len() * (small_char_width + small_padding);
            let info_x = (width - info_text_width) / 2;
            let info_y = height - 60; // Near bottom

            self.draw_text(
                buffer,
                width,
                height,
                bytes_per_row,
                &characters,
                &char_map,
                info_text,
                info_x,
                info_y,
                small_char_width,
                small_char_height,
                small_padding,
                [150, 50, 50],
            ); // Red color
        }
    }

    // Helper to draw text with the bitmap font
    fn draw_text(
        &self,
        buffer: &mut Vec<u8>,
        width: usize,
        height: usize,
        bytes_per_row: usize,
        characters: &[[[u8; 5]; 5]],
        char_map: &std::collections::HashMap<char, usize>,
        text: &str,
        start_x: usize,
        start_y: usize,
        char_width: usize,
        char_height: usize,
        char_padding: usize,
        color: [u8; 3],
    ) {
        for (i, c) in text.chars().enumerate() {
            // Get character bitmap or use space for unknown characters
            let char_idx = char_map.get(&c).copied().unwrap_or(10); // Default to space
            let bitmap = &characters[char_idx];

            // Character position
            let char_x = start_x + i * (char_width + char_padding);

            // Scale the 5x5 bitmap to the desired size
            let scale_x = char_width / 5;
            let scale_y = char_height / 5;

            // Draw the character
            for (y_idx, row) in bitmap.iter().enumerate() {
                for (x_idx, &pixel) in row.iter().enumerate() {
                    if pixel == 1 {
                        // Fill the scaled pixel area
                        for sy in 0..scale_y {
                            for sx in 0..scale_x {
                                let x = char_x + x_idx * scale_x + sx;
                                let y = start_y + y_idx * scale_y + sy;

                                // Skip if outside buffer bounds
                                if x >= width || y >= height {
                                    continue;
                                }

                                let idx = y * bytes_per_row + x * 4;
                                if idx + 3 < buffer.len() {
                                    buffer[idx] = color[0]; // Red
                                    buffer[idx + 1] = color[1]; // Green
                                    buffer[idx + 2] = color[2]; // Blue
                                    buffer[idx + 3] = 255; // Alpha
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Add debug borders and corner markers to the source pattern
    fn add_debug_borders(
        &self,
        buffer: &mut Vec<u8>,
        width: usize,
        height: usize,
        bytes_per_row: usize,
    ) {
        // Border thickness
        let border_thickness = 3;
        // Corner box size
        let corner_size = 15;

        // Draw border - top and bottom edges
        for y in 0..border_thickness {
            // Top edge
            for x in 0..width {
                let idx = y * bytes_per_row + x * 4;
                buffer[idx] = 255; // Red
                buffer[idx + 1] = 0; // Green
                buffer[idx + 2] = 0; // Blue
                buffer[idx + 3] = 255; // Alpha
            }

            // Bottom edge
            if height > border_thickness {
                for x in 0..width {
                    let idx = (height - 1 - y) * bytes_per_row + x * 4;
                    buffer[idx] = 255; // Red
                    buffer[idx + 1] = 0; // Green
                    buffer[idx + 2] = 0; // Blue
                    buffer[idx + 3] = 255; // Alpha
                }
            }
        }

        // Draw border - left and right edges
        for x in 0..border_thickness {
            // Left edge
            for y in 0..height {
                let idx = y * bytes_per_row + x * 4;
                buffer[idx] = 255; // Red
                buffer[idx + 1] = 0; // Green
                buffer[idx + 2] = 0; // Blue
                buffer[idx + 3] = 255; // Alpha
            }

            // Right edge
            if width > border_thickness {
                for y in 0..height {
                    let idx = y * bytes_per_row + (width - 1 - x) * 4;
                    buffer[idx] = 255; // Red
                    buffer[idx + 1] = 0; // Green
                    buffer[idx + 2] = 0; // Blue
                    buffer[idx + 3] = 255; // Alpha
                }
            }
        }

        // Draw colored corner boxes

        // Top-left corner box (Red)
        for y in 0..corner_size {
            for x in 0..corner_size {
                let idx = y * bytes_per_row + x * 4;
                buffer[idx] = 255; // Red
                buffer[idx + 1] = 0; // Green
                buffer[idx + 2] = 0; // Blue
                buffer[idx + 3] = 255; // Alpha
            }
        }

        // Top-right corner box (Green)
        if width > corner_size {
            for y in 0..corner_size {
                for x in 0..corner_size {
                    let idx = y * bytes_per_row + (width - corner_size + x) * 4;
                    buffer[idx] = 0; // Red
                    buffer[idx + 1] = 255; // Green
                    buffer[idx + 2] = 0; // Blue
                    buffer[idx + 3] = 255; // Alpha
                }
            }
        }

        // Bottom-left corner box (Blue)
        if height > corner_size {
            for y in 0..corner_size {
                for x in 0..corner_size {
                    let idx = (height - corner_size + y) * bytes_per_row + x * 4;
                    buffer[idx] = 0; // Red
                    buffer[idx + 1] = 0; // Green
                    buffer[idx + 2] = 255; // Blue
                    buffer[idx + 3] = 255; // Alpha
                }
            }
        }

        // Bottom-right corner box (Yellow)
        if width > corner_size && height > corner_size {
            for y in 0..corner_size {
                for x in 0..corner_size {
                    let idx =
                        (height - corner_size + y) * bytes_per_row + (width - corner_size + x) * 4;
                    buffer[idx] = 255; // Red
                    buffer[idx + 1] = 255; // Green
                    buffer[idx + 2] = 0; // Blue
                    buffer[idx + 3] = 255; // Alpha
                }
            }
        }
    }

    fn render(&self) -> Option<Retained<NSImage>> {
        let (viewport_width, viewport_height) = self.get_viewport_size();

        // Create a new image of the viewport size
        let size = NSSize::new(viewport_width as f64, viewport_height as f64);
        let alloc = NSImage::alloc();
        let image = unsafe { NSImage::initWithSize(alloc, size) };

        // Create a bitmap representation for the viewport
        let alloc = NSBitmapImageRep::alloc();
        let color_space_name = ns_string!("NSDeviceRGBColorSpace");
        let bits_per_component = 8;
        let bytes_per_row = viewport_width * 4; // RGBA format

        let rep = unsafe {
            let planes: *const *mut u8 = std::ptr::null();
            let rep: Retained<NSBitmapImageRep> = msg_send![alloc,
                initWithBitmapDataPlanes: planes,
                pixelsWide: viewport_width as isize,
                pixelsHigh: viewport_height as isize,
                bitsPerSample: bits_per_component as isize,
                samplesPerPixel: 4 as isize,
                hasAlpha: true,
                isPlanar: false,
                colorSpaceName: &*color_space_name,
                bytesPerRow: bytes_per_row as isize,
                bitsPerPixel: 32 as isize
            ];

            rep
        };

        // Get bitmap data buffer
        let buffer: *mut u8 = unsafe { msg_send![&*rep, bitmapData] };

        if buffer.is_null() {
            println!("Failed to get bitmap data");
            return None;
        }

        // Apply zooming and panning
        if let Some(source) = &self.source_pattern {
            unsafe {
                // Calculate scaling factor and starting position
                let scale_factor = 1.0 / self.zoom_level;
                let start_src_x = (self.view_x * scale_factor) as usize;
                let start_src_y = (self.view_y * scale_factor) as usize;

                for y in 0..viewport_height {
                    for x in 0..viewport_width {
                        let dst_idx = (y * bytes_per_row + x * 4) as isize;

                        // Map viewport position to source pattern coordinates
                        let src_x = start_src_x + (x as f64 * scale_factor) as usize;
                        let src_y = start_src_y + (y as f64 * scale_factor) as usize;

                        // Clamp source coordinates to valid range
                        let src_x_clamped = src_x.min(source.width - 1);
                        let src_y_clamped = src_y.min(source.height - 1);

                        // Calculate source index
                        let src_idx = src_y_clamped * source.bytes_per_row + src_x_clamped * 4;

                        // Copy pixel from source to destination
                        if src_idx + 3 < source.buffer.len() {
                            *buffer.offset(dst_idx) = source.buffer[src_idx]; // Red
                            *buffer.offset(dst_idx + 1) = source.buffer[src_idx + 1]; // Green
                            *buffer.offset(dst_idx + 2) = source.buffer[src_idx + 2]; // Blue
                            *buffer.offset(dst_idx + 3) = source.buffer[src_idx + 3];
                        // Alpha
                        } else {
                            // If out of bounds, set to a distinctive color (purple)
                            *buffer.offset(dst_idx) = 128; // Red
                            *buffer.offset(dst_idx + 1) = 0; // Green
                            *buffer.offset(dst_idx + 2) = 128; // Blue
                            *buffer.offset(dst_idx + 3) = 255; // Alpha
                        }
                    }
                }
            }
        }

        // Add the bitmap representation to the image
        unsafe { image.addRepresentation(&rep) };

        Some(image)
    }
}

// Define a custom image view subclass that forwards mouse events to our app delegate
define_class!(
    #[unsafe(super = NSImageView)]
    #[thread_kind = MainThreadOnly]
    #[name = "CustomImageView"]
    #[derive(Debug)]
    struct CustomImageView;

    unsafe impl NSObjectProtocol for CustomImageView {}

    impl CustomImageView {
        #[unsafe(method(mouseDown:))]
        fn mouseDown(&self, event: &NSEvent) {
            // Pass the event to the app delegate
            if let Some(delegate) = self.get_app_delegate() {
                unsafe {
                    let _: Bool = msg_send![delegate, mouseDown: event];
                }
            }

            // Call super implementation
            unsafe {
                let _: () = msg_send![super(self), mouseDown: event];
            }
        }

        #[unsafe(method(mouseDragged:))]
        fn mouseDragged(&self, event: &NSEvent) {
            // Pass the event to the app delegate
            if let Some(delegate) = self.get_app_delegate() {
                unsafe {
                    let _: Bool = msg_send![delegate, mouseDragged: event];
                }
            }

            // Call super implementation
            unsafe {
                let _: () = msg_send![super(self), mouseDragged: event];
            }
        }

        #[unsafe(method(mouseUp:))]
        fn mouseUp(&self, event: &NSEvent) {
            // Pass the event to the app delegate
            if let Some(delegate) = self.get_app_delegate() {
                unsafe {
                    let _: Bool = msg_send![delegate, mouseUp: event];
                }
            }

            // Call super implementation
            unsafe {
                let _: () = msg_send![super(self), mouseUp: event];
            }
        }
    }
);

impl CustomImageView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm);
        unsafe {
            let obj: Retained<Self> = msg_send![this, initWithFrame: frame];
            obj
        }
    }

    fn get_app_delegate(&self) -> Option<&AnyObject> {
        let mtm = self.mtm();
        let app = NSApplication::sharedApplication(mtm);

        unsafe {
            let delegate: *const AnyObject = msg_send![&*app, delegate];
            if delegate.is_null() {
                None
            } else {
                Some(&*delegate)
            }
        }
    }
}

// Define the app delegate with ivars
#[derive(Debug, Default)]
struct AppDelegateIvars {
    window: OnceCell<Retained<NSWindow>>,
    scroll_view: OnceCell<Retained<NSScrollView>>,
    image_view: OnceCell<Retained<CustomImageView>>,
    selected_file_path: RefCell<Option<Retained<NSURL>>>,
    decoded_image: RefCell<Option<Retained<NSImage>>>,
    renderer: RefCell<Option<Arc<Mutex<ImageRenderer>>>>,
    zoom_slider: OnceCell<Retained<NSSlider>>,
    last_mouse_location: RefCell<NSPoint>,
    is_panning: RefCell<bool>,
}

define_class!(
    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - `AppDelegate` does not implement `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "AppDelegate"]
    #[ivars = AppDelegateIvars]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn applicationDidFinishLaunching(&self, _notification: &NSNotification) {
            println!("DEBUG: Application did finish launching");

            let mtm = self.mtm();

            // Create a window
            let window = self.create_window(mtm);
            let _ = self.ivars().window.set(window.clone());

            // Set up the window
            window.setTitle(ns_string!("JP2 Viewer"));
            window.center();

            // Create scroll view and image view
            self.setup_image_view(&window, mtm);

            // Create zoom controls
            self.setup_zoom_controls(&window, mtm);

            // Add buttons
            self.add_buttons(&window, mtm);

            // Set up mouse event handling
            self.setup_mouse_handling(&window);

            // Activate the application first to ensure it's frontmost
            let app = NSApplication::sharedApplication(mtm);
            unsafe { app.activate() };

            // Then make window key and visible
            window.makeKeyAndOrderFront(None);
        }
    }

    unsafe impl NSWindowDelegate for AppDelegate {
        #[unsafe(method(windowWillClose:))]
        fn windowWillClose(&self, _notification: &NSNotification) {
            // Quit the application when the window is closed
            let mtm = self.mtm();
            let app = NSApplication::sharedApplication(mtm);
            unsafe { app.terminate(None) };
        }
    }

    // Add custom methods for our delegate
    impl AppDelegate {
        #[unsafe(method(openFile:))]
        fn openFile(&self, _sender: Option<&NSObject>) -> Bool {
            println!("DEBUG: Opening file dialog");

            let mtm = self.mtm();
            let panel = unsafe { objc2_app_kit::NSOpenPanel::openPanel(mtm) };

            unsafe {
                panel.setCanChooseFiles(true);
                panel.setCanChooseDirectories(false);
                panel.setAllowsMultipleSelection(false);

                // Set up allowed file types
                let types = NSArray::from_slice(&[ns_string!("jp2")]);
                panel.setAllowedFileTypes(Some(&types));

                // Show the panel
                let response = panel.runModal();

                // Check response (1 = NSModalResponseOK)
                if response == 1 {
                    let urls = panel.URLs();
                    if let Some(url) = urls.firstObject() {
                        // Use the URL for debugging but don't try to extract filename directly
                        println!("DEBUG: Selected file: {:?}", url);

                        // Store the path
                        *self.ivars().selected_file_path.borrow_mut() = Some(url.clone());

                        // Extract the actual filename from the URL
                        let filename = {
                            // Log the raw URL for debugging
                            println!("DEBUG: Raw URL: {:?}", url);

                            // Get URL string from NSURLs path() method which is safer than debug formatting
                            let url_path = {
                                if let Some(path) = url.path().as_deref() {
                                    let ns_string = path.to_owned();
                                    // Convert NSString to Rust String - use display instead of debug
                                    format!("{}", &*ns_string)
                                } else {
                                    "unknown_path".to_string()
                                }
                            };

                            println!("DEBUG: Extracted path: {}", url_path);

                            // Extract just the filename portion
                            let filename = url_path.split('/').last()
                                .unwrap_or("JP2 File")
                                .to_string();

                            println!("DEBUG: Extracted filename: {}", filename);
                            Some(filename)
                        };

                        // Show the "Coming Soon" text pattern since JP2 loading is not implemented yet
                        println!("DEBUG: Showing Coming Soon text pattern for JP2 file: {:?}", filename);

                        // Check if we already have a renderer
                        let need_new_renderer = self.ivars().renderer.borrow().is_none();

                        if need_new_renderer {
                            // Create text pattern with renderer
                            let width = 800;
                            let height = 600;

                            let mut renderer = ImageRenderer::new(PatternType::Text, width, height);
                            renderer.set_text(Some("COMING SOON".to_string()), filename);

                            let renderer = Arc::new(Mutex::new(renderer));
                            *self.ivars().renderer.borrow_mut() = Some(renderer.clone());
                        } else {
                            // Update existing renderer to use text pattern
                            if let Some(renderer) = self.ivars().renderer.borrow().as_ref() {
                                let mut renderer_guard = renderer.lock().unwrap();
                                renderer_guard.change_pattern_type(PatternType::Text);
                                renderer_guard.set_text(Some("COMING SOON".to_string()), filename);
                            }
                        }

                        // Render the image with the current renderer
                        if let Some(renderer) = self.ivars().renderer.borrow().as_ref() {
                            let image = {
                                let renderer_guard = renderer.lock().unwrap();
                                renderer_guard.render()
                            };

                            if let Some(image) = image {
                                *self.ivars().decoded_image.borrow_mut() = Some(image.clone());

                                // Display the image
                                unsafe {
                                    let _: Bool = msg_send![self, handleDisplayImage];
                                }
                                return Bool::YES;
                            }
                        }
                    }
                }
            }

            Bool::NO
        }

        #[unsafe(method(createGradient:))]
        fn createGradient(&self, _sender: Option<&NSObject>) -> Bool {
            println!("DEBUG: Creating gradient image");

            // Check if we already have a renderer
            let need_new_renderer = self.ivars().renderer.borrow().is_none();

            if need_new_renderer {
                // Create a new gradient image with renderer
                let width = 800;
                let height = 600;

                let renderer = Arc::new(Mutex::new(
                    ImageRenderer::new(PatternType::Gradient, width, height)
                ));

                *self.ivars().renderer.borrow_mut() = Some(renderer.clone());
            } else {
                // Update existing renderer to use gradient pattern
                if let Some(renderer) = self.ivars().renderer.borrow().as_ref() {
                    let mut renderer_guard = renderer.lock().unwrap();
                    renderer_guard.change_pattern_type(PatternType::Gradient);
                }
            }

            // Render the image with the current renderer
            if let Some(renderer) = self.ivars().renderer.borrow().as_ref() {
                let image = {
                    let renderer_guard = renderer.lock().unwrap();
                    renderer_guard.render()
                };

                if let Some(image) = image {
                    // Store the image in the delegate
                    *self.ivars().decoded_image.borrow_mut() = Some(image.clone());

                    // Display the image
                    unsafe {
                        let _: Bool = msg_send![self, handleDisplayImage];
                    }
                    return Bool::YES;
                }
            }

            Bool::NO
        }

        #[unsafe(method(createCheckerboard:))]
        fn createCheckerboard(&self, _sender: Option<&NSObject>) -> Bool {
            println!("DEBUG: Creating checkerboard image");

            // Check if we already have a renderer
            let need_new_renderer = self.ivars().renderer.borrow().is_none();

            if need_new_renderer {
                // Create a new checkerboard image with renderer
                let width = 800;
                let height = 600;

                let renderer = Arc::new(Mutex::new(
                    ImageRenderer::new(PatternType::Checkerboard, width, height)
                ));

                *self.ivars().renderer.borrow_mut() = Some(renderer.clone());
            } else {
                // Update existing renderer to use checkerboard pattern
                if let Some(renderer) = self.ivars().renderer.borrow().as_ref() {
                    let mut renderer_guard = renderer.lock().unwrap();
                    renderer_guard.change_pattern_type(PatternType::Checkerboard);
                }
            }

            // Render the image with the current renderer
            if let Some(renderer) = self.ivars().renderer.borrow().as_ref() {
                let image = {
                    let renderer_guard = renderer.lock().unwrap();
                    renderer_guard.render()
                };

                if let Some(image) = image {
                    // Store the image in the delegate
                    *self.ivars().decoded_image.borrow_mut() = Some(image.clone());

                    // Display the image
                    unsafe {
                        let _: Bool = msg_send![self, handleDisplayImage];
                    }
                    return Bool::YES;
                }
            }

            Bool::NO
        }

        #[unsafe(method(handleDisplayImage))]
        unsafe fn handleDisplayImage(&self) -> Bool {
            println!("DEBUG: Starting display_image");

            let image_view = match self.ivars().image_view.get() {
                Some(view) => view,
                None => {
                    println!("DEBUG: No image view available");
                    return Bool::NO;
                }
            };

            let decoded_image = self.ivars().decoded_image.borrow();
            let image = match decoded_image.as_ref() {
                Some(img) => img,
                None => {
                    println!("DEBUG: No image to display");
                    return Bool::NO;
                }
            };

            unsafe {
                // Set the image
                image_view.setImage(Some(image));

                // Update image view size to match the image size
                let image_size = image.size();
                let frame = NSRect::new(NSPoint::new(0.0, 0.0), image_size);
                image_view.setFrame(frame);
            }

            // Adjust scroll view content size
            if let Some(scroll_view) = self.ivars().scroll_view.get() {
                unsafe {
                    scroll_view.documentView().unwrap().setFrame(image_view.frame());
                    scroll_view.setNeedsDisplay(true);
                }
            }

            println!("DEBUG: Updated image view");
            Bool::YES
        }

        #[unsafe(method(zoomChanged:))]
        fn zoomChanged(&self, sender: Option<&NSObject>) -> Bool {
            if let Some(obj) = sender {
                let slider_value: f64 = unsafe { msg_send![obj, doubleValue] };
                println!("DEBUG: Zoom changed to {}", slider_value);

                if let Some(renderer) = self.ivars().renderer.borrow().as_ref() {
                    // Update zoom level in renderer
                    {
                        let mut renderer_guard = renderer.lock().unwrap();
                        renderer_guard.set_zoom(slider_value);
                    }

                    // Re-render the image with new zoom
                    let image = {
                        let renderer_guard = renderer.lock().unwrap();
                        renderer_guard.render()
                    };

                    if let Some(image) = image {
                        *self.ivars().decoded_image.borrow_mut() = Some(image.clone());

                        // Update the display
                        unsafe {
                            let _: Bool = msg_send![self, handleDisplayImage];
                        }
                        return Bool::YES;
                    }
                }
            }

            Bool::NO
        }

        #[unsafe(method(mouseDown:))]
        fn mouseDown(&self, event: &NSEvent) -> Bool {
            println!("DEBUG: Mouse down received");
            // Start panning mode
            *self.ivars().is_panning.borrow_mut() = true;

            // Store initial mouse location
            let location = unsafe { event.locationInWindow() };
            *self.ivars().last_mouse_location.borrow_mut() = location;

            Bool::YES
        }

        #[unsafe(method(mouseDragged:))]
        fn mouseDragged(&self, event: &NSEvent) -> Bool {
            println!("DEBUG: Mouse dragged");
            if *self.ivars().is_panning.borrow() {
                let current_location = unsafe { event.locationInWindow() };
                let last_location = *self.ivars().last_mouse_location.borrow();

                // Calculate the delta in screen coordinates
                let delta_x = current_location.x - last_location.x;
                let delta_y = current_location.y - last_location.y;

                // Update renderer view position
                if let Some(renderer) = self.ivars().renderer.borrow().as_ref() {
                    {
                        let mut renderer_guard = renderer.lock().unwrap();
                        let current_x = renderer_guard.view_x;
                        let current_y = renderer_guard.view_y;

                        renderer_guard.set_pan(
                            current_x - delta_x,
                            current_y - delta_y
                        );
                    }

                    // Re-render with new view position
                    let image = {
                        let renderer_guard = renderer.lock().unwrap();
                        renderer_guard.render()
                    };

                    if let Some(image) = image {
                        *self.ivars().decoded_image.borrow_mut() = Some(image.clone());

                        // Update the display
                        unsafe {
                            let _: Bool = msg_send![self, handleDisplayImage];
                        }
                    }
                }

                // Update the last location
                *self.ivars().last_mouse_location.borrow_mut() = current_location;
                return Bool::YES;
            }

            Bool::NO
        }

        #[unsafe(method(mouseUp:))]
        fn mouseUp(&self, _event: &NSEvent) -> Bool {
            println!("DEBUG: Mouse up received");
            // End panning mode
            *self.ivars().is_panning.borrow_mut() = false;
            Bool::YES
        }
    }
);

// Implement custom methods for AppDelegate
impl AppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(AppDelegateIvars::default());
        unsafe { msg_send![super(this), init] }
    }

    fn create_window(&self, mtm: MainThreadMarker) -> Retained<NSWindow> {
        let window_frame = NSRect::new(NSPoint::new(100., 100.), NSSize::new(800., 600.));
        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Resizable
            | NSWindowStyleMask::Miniaturizable;

        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                window_frame,
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };

        // Important: prevent automatic closing from releasing the window
        // This is needed when not using a window controller
        unsafe { window.setReleasedWhenClosed(false) };

        window
    }

    fn setup_image_view(&self, window: &NSWindow, mtm: MainThreadMarker) {
        let content_view = window.contentView().unwrap();
        let content_frame = content_view.bounds();

        // Calculate the main view frame, leaving room for controls at the bottom
        let controls_height = 60.0;
        let main_view_frame = NSRect::new(
            NSPoint::new(0.0, controls_height),
            NSSize::new(
                content_frame.size.width,
                content_frame.size.height - controls_height,
            ),
        );

        // Create a scroll view
        let scroll_view =
            unsafe { NSScrollView::initWithFrame(NSScrollView::alloc(mtm), main_view_frame) };

        unsafe {
            scroll_view.setHasVerticalScroller(true);
            scroll_view.setHasHorizontalScroller(true);
            scroll_view.setAutoresizingMask(
                NSAutoresizingMaskOptions::ViewWidthSizable
                    | NSAutoresizingMaskOptions::ViewHeightSizable,
            );

            // Create our custom image view for the document view
            let frame = NSRect::ZERO;
            let new_image_view = CustomImageView::new(mtm, frame);

            // Configure image view properties
            new_image_view.setImageScaling(NSImageScaling::ScaleProportionallyDown);

            // Set the image view as the document view
            scroll_view.setDocumentView(Some(&*new_image_view));

            // Add the scroll view to the content view
            content_view.addSubview(&scroll_view);

            // Store the views
            let _ = self.ivars().scroll_view.set(scroll_view.clone());
            let _ = self.ivars().image_view.set(new_image_view.clone());
        }
    }

    fn setup_zoom_controls(&self, window: &NSWindow, mtm: MainThreadMarker) {
        let content_view = window.contentView().unwrap();

        // Create a slider for zoom control
        let slider_frame = NSRect::new(NSPoint::new(530., 25.), NSSize::new(180., 30.));
        let slider = unsafe { NSSlider::initWithFrame(NSSlider::alloc(mtm), slider_frame) };

        unsafe {
            // Configure slider properties
            slider.setMinValue(0.1);
            slider.setMaxValue(5.0);
            slider.setDoubleValue(1.0);

            // Set number of tick marks directly using msg_send - use i64 (long) instead of i32
            let _: () = msg_send![&*slider, setNumberOfTickMarks: 9i64];
            let _: () = msg_send![&*slider, setAllowsTickMarkValuesOnly: false];

            // Set action and target
            slider.setAction(Some(sel!(zoomChanged:)));
            let target: Option<&AnyObject> = Some(self.as_ref());
            slider.setTarget(target);

            // Add to content view
            content_view.addSubview(&slider);

            // Store the slider
            let _ = self.ivars().zoom_slider.set(slider.clone());
        }
    }

    fn add_buttons(&self, window: &NSWindow, mtm: MainThreadMarker) {
        // Create Open JP2 button
        let open_button_frame = NSRect::new(NSPoint::new(20., 20.), NSSize::new(100., 30.));
        let open_button =
            unsafe { NSButton::initWithFrame(NSButton::alloc(mtm), open_button_frame) };

        unsafe {
            open_button.setTitle(ns_string!("Open JP2"));
            open_button.setBezelStyle(NSBezelStyle::Rounded);
            open_button.setAction(Some(sel!(openFile:)));

            // Convert self to AnyObject for target
            let target: Option<&AnyObject> = Some(self.as_ref());
            open_button.setTarget(target);

            let content_view = window.contentView().unwrap();
            content_view.addSubview(&open_button);
        }

        // Create Gradient button
        let gradient_button_frame = NSRect::new(NSPoint::new(140., 20.), NSSize::new(100., 30.));
        let gradient_button =
            unsafe { NSButton::initWithFrame(NSButton::alloc(mtm), gradient_button_frame) };

        unsafe {
            gradient_button.setTitle(ns_string!("Gradient"));
            gradient_button.setBezelStyle(NSBezelStyle::Rounded);
            gradient_button.setAction(Some(sel!(createGradient:)));

            // Convert self to AnyObject for target
            let target: Option<&AnyObject> = Some(self.as_ref());
            gradient_button.setTarget(target);

            let content_view = window.contentView().unwrap();
            content_view.addSubview(&gradient_button);
        }

        // Create Checkerboard button
        let checkerboard_button_frame =
            NSRect::new(NSPoint::new(260., 20.), NSSize::new(100., 30.));
        let checkerboard_button =
            unsafe { NSButton::initWithFrame(NSButton::alloc(mtm), checkerboard_button_frame) };

        unsafe {
            checkerboard_button.setTitle(ns_string!("Checkerboard"));
            checkerboard_button.setBezelStyle(NSBezelStyle::Rounded);
            checkerboard_button.setAction(Some(sel!(createCheckerboard:)));

            // Convert self to AnyObject for target
            let target: Option<&AnyObject> = Some(self.as_ref());
            checkerboard_button.setTarget(target);

            let content_view = window.contentView().unwrap();
            content_view.addSubview(&checkerboard_button);
        }
    }

    fn setup_mouse_handling(&self, _window: &NSWindow) {
        // Initial values
        *self.ivars().is_panning.borrow_mut() = false;
        *self.ivars().last_mouse_location.borrow_mut() = NSPoint::new(0.0, 0.0);

        // All mouse handling is now done through our CustomImageView subclass
        // that forwards events to our AppDelegate
        if let Some(window) = self.ivars().window.get() {
            window.setAcceptsMouseMovedEvents(true);
        }
    }
}

fn main() {
    // Initialize on the main thread
    let mtm = MainThreadMarker::new().expect("Not running on main thread");

    // Get the shared application instance
    let app = NSApplication::sharedApplication(mtm);

    // Set the activation policy
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    // Create our app delegate
    let delegate = AppDelegate::new(mtm);

    // Set the delegate
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    // Activation is now done in applicationDidFinishLaunching
    // to properly sequence window visibility

    println!("DEBUG: Starting application run loop");
    app.run();
}
