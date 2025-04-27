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
}

// Enum to represent different pattern types
#[derive(Debug)]
enum PatternType {
    Checkerboard,
    Gradient,
}

impl ImageRenderer {
    fn new(pattern_type: PatternType, width: usize, height: usize) -> Self {
        Self {
            source_width: width,
            source_height: height,
            zoom_level: 1.0,
            view_x: 0.0,
            view_y: 0.0,
            pattern_type,
        }
    }

    fn set_zoom(&mut self, zoom: f64) {
        self.zoom_level = zoom.max(0.1).min(10.0);
    }

    fn set_pan(&mut self, x: f64, y: f64) {
        self.view_x = x;
        self.view_y = y;
    }

    fn get_viewport_size(&self) -> (usize, usize) {
        let width = (self.source_width as f64 * self.zoom_level) as usize;
        let height = (self.source_height as f64 * self.zoom_level) as usize;
        (width, height)
    }

    fn render(&self) -> Option<Retained<NSImage>> {
        let (width, height) = self.get_viewport_size();

        match self.pattern_type {
            PatternType::Checkerboard => self.create_checkerboard_image(width, height),
            PatternType::Gradient => self.create_gradient_image(width, height),
        }
    }

    // Moved from AppDelegate and adapted for zooming/panning
    fn create_gradient_image(&self, width: usize, height: usize) -> Option<Retained<NSImage>> {
        let size = NSSize::new(width as f64, height as f64);

        let alloc = NSImage::alloc();
        let image = unsafe { NSImage::initWithSize(alloc, size) };

        // Create a bitmap representation
        let alloc = NSBitmapImageRep::alloc();
        let color_space_name = ns_string!("NSDeviceRGBColorSpace");

        let bits_per_component = 8;
        let bytes_per_row = width * 4; // RGBA format

        let rep = unsafe {
            let planes: *const *mut u8 = std::ptr::null();
            let rep: Retained<NSBitmapImageRep> = msg_send![alloc,
                initWithBitmapDataPlanes: planes,
                pixelsWide: width as isize,
                pixelsHigh: height as isize,
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

        // Fill with a gradient
        unsafe {
            let bytes_per_row = width * 4;

            // Calculate viewport to source image mapping
            let scale_factor = 1.0 / self.zoom_level;
            let start_src_x = (self.view_x * scale_factor) as usize;
            let start_src_y = (self.view_y * scale_factor) as usize;

            for y in 0..height {
                for x in 0..width {
                    let buffer_index = (y * bytes_per_row + x * 4) as isize;

                    // Map viewport position to source image coordinates
                    let src_x = start_src_x + (x as f64 * scale_factor) as usize;
                    let src_y = start_src_y + (y as f64 * scale_factor) as usize;

                    // Use clamped source coordinates to generate gradient
                    let src_x_clamped = src_x.min(self.source_width - 1);
                    let src_y_clamped = src_y.min(self.source_height - 1);

                    // Create a blue to white gradient
                    let r = ((src_x_clamped as f64) / (self.source_width as f64) * 255.0) as u8;
                    let g = ((src_y_clamped as f64) / (self.source_height as f64) * 255.0) as u8;
                    let b = 200u8;

                    *buffer.offset(buffer_index) = r; // Red
                    *buffer.offset(buffer_index + 1) = g; // Green
                    *buffer.offset(buffer_index + 2) = b; // Blue
                    *buffer.offset(buffer_index + 3) = 255; // Alpha
                }
            }
        }

        // Add the bitmap representation to the image
        unsafe { image.addRepresentation(&rep) };

        Some(image)
    }

    // Moved from AppDelegate and adapted for zooming/panning
    fn create_checkerboard_image(&self, width: usize, height: usize) -> Option<Retained<NSImage>> {
        let size = NSSize::new(width as f64, height as f64);

        let alloc = NSImage::alloc();
        let image = unsafe { NSImage::initWithSize(alloc, size) };

        // Create a bitmap representation
        let alloc = NSBitmapImageRep::alloc();
        let color_space_name = ns_string!("NSDeviceRGBColorSpace");

        let bits_per_component = 8;
        let bytes_per_row = width * 4; // RGBA format

        let rep = unsafe {
            let planes: *const *mut u8 = std::ptr::null();
            let rep: Retained<NSBitmapImageRep> = msg_send![alloc,
                initWithBitmapDataPlanes: planes,
                pixelsWide: width as isize,
                pixelsHigh: height as isize,
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

        // Fill with a checkerboard pattern
        unsafe {
            let bytes_per_row = width * 4;
            let square_size = 20; // Size of each checkerboard square

            // Calculate viewport to source image mapping
            let scale_factor = 1.0 / self.zoom_level;
            let start_src_x = (self.view_x * scale_factor) as usize;
            let start_src_y = (self.view_y * scale_factor) as usize;

            for y in 0..height {
                for x in 0..width {
                    let buffer_index = (y * bytes_per_row + x * 4) as isize;

                    // Map viewport position to source image coordinates
                    let src_x = start_src_x + (x as f64 * scale_factor) as usize;
                    let src_y = start_src_y + (y as f64 * scale_factor) as usize;

                    // Determine if this pixel should be black or white
                    let is_white = ((src_x / square_size) + (src_y / square_size)) % 2 == 0;

                    let color = if is_white { 255u8 } else { 0u8 };

                    *buffer.offset(buffer_index) = color; // Red
                    *buffer.offset(buffer_index + 1) = color; // Green
                    *buffer.offset(buffer_index + 2) = color; // Blue
                    *buffer.offset(buffer_index + 3) = 255; // Alpha
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
                        println!("DEBUG: Selected file: {:?}", url);

                        // Store the path
                        *self.ivars().selected_file_path.borrow_mut() = Some(url.clone());

                        // Create checkerboard pattern with renderer
                        let width = 800;
                        let height = 600;

                        let renderer = Arc::new(Mutex::new(
                            ImageRenderer::new(PatternType::Checkerboard, width, height)
                        ));

                        *self.ivars().renderer.borrow_mut() = Some(renderer.clone());

                        // Render the image
                        let image = {
                            let renderer_guard = renderer.lock().unwrap();
                            renderer_guard.render()
                        };

                        if let Some(image) = image {
                            *self.ivars().decoded_image.borrow_mut() = Some(image.clone());

                            // Display the image
                            let _: Bool = msg_send![self, handleDisplayImage];
                            return Bool::YES;
                        }
                    }
                }
            }

            Bool::NO
        }

        #[unsafe(method(createGradient:))]
        fn createGradient(&self, _sender: Option<&NSObject>) -> Bool {
            println!("DEBUG: Creating gradient image");

            // Create a gradient image with renderer
            let width = 800;
            let height = 600;

            let renderer = Arc::new(Mutex::new(
                ImageRenderer::new(PatternType::Gradient, width, height)
            ));

            *self.ivars().renderer.borrow_mut() = Some(renderer.clone());

            // Render the image
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
