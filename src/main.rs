#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case)]

use std::cell::{OnceCell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, ProtocolObject};
use objc2::AnyThread;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSAutoresizingMaskOptions,
    NSBackingStoreType, NSBezelStyle, NSBitmapImageRep, NSButton, NSImage, NSImageScaling,
    NSImageView, NSWindow, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::NSString;
use objc2_foundation::{
    ns_string, NSArray, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSURL,
};

// Define the app delegate with ivars
#[derive(Debug, Default)]
struct AppDelegateIvars {
    window: OnceCell<Retained<NSWindow>>,
    image_view: OnceCell<Retained<NSImageView>>,
    selected_file_path: RefCell<Option<Retained<NSURL>>>,
    decoded_image: RefCell<Option<Retained<NSImage>>>,
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

            // Activate the application first to ensure it's frontmost
            let app = NSApplication::sharedApplication(mtm);
            unsafe { app.activate() };

            // Then make window key and visible
            window.makeKeyAndOrderFront(None);

            // Create and add a button to the window
            self.add_open_button(&window, mtm);
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

                        // Load and display the JP2 file
                        let _: Bool = msg_send![self, handleJP2File];
                        return Bool::YES;
                    }
                }
            }

            Bool::NO
        }

        #[unsafe(method(handleJP2File))]
        unsafe fn handleJP2File(&self) -> Bool {
            println!("DEBUG: Loading JP2 file");

            let selected_file = self.ivars().selected_file_path.borrow();
            let url = match selected_file.as_ref() {
                Some(url) => url,
                None => {
                    println!("DEBUG: No file selected");
                    return Bool::NO;
                }
            };

            // Get the path from the URL
            let path_str = unsafe { url.path() }.unwrap_or_else(|| NSString::new() );
            let path = path_str.to_string();
            println!("DEBUG: Loading JP2 file: {}", path);

            // For now, create a placeholder image
            let width = 800;
            let height = 600;
            println!("DEBUG: Creating placeholder image");

            let image = self.create_placeholder_image(width, height);

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

            let window = match self.ivars().window.get() {
                Some(win) => win,
                None => {
                    println!("DEBUG: No window available");
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

            let content_view = window.contentView().unwrap();

            // Create an image view if it doesn't exist
            if self.ivars().image_view.get().is_none() {
                println!("DEBUG: Creating new image view");

                let mtm = self.mtm();
                let frame = NSRect::ZERO;
                let new_image_view = unsafe { NSImageView::initWithFrame(NSImageView::alloc(mtm), frame) };

                unsafe {
                    // Configure image view properties
                    new_image_view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                    new_image_view.setAutoresizingMask(
                        NSAutoresizingMaskOptions::ViewWidthSizable |
                        NSAutoresizingMaskOptions::ViewHeightSizable
                    );

                    // Add the image view to the content view
                    content_view.addSubview(&new_image_view);

                    // Store the image view
                    let _ = self.ivars().image_view.set(new_image_view.clone());

                    // Set the image
                    new_image_view.setImage(Some(image));

                    // Resize the image view to fit the content view
                    let content_frame = content_view.bounds();
                    new_image_view.setFrame(content_frame);
                }

                Bool::YES
            } else {
                // Update existing image view
                let image_view = self.ivars().image_view.get().unwrap();
                unsafe {
                    image_view.setImage(Some(image));
                }
                println!("DEBUG: Updated existing image view");

                Bool::YES
            }
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

    fn add_open_button(&self, window: &NSWindow, mtm: MainThreadMarker) {
        let button_frame = NSRect::new(NSPoint::new(350., 30.), NSSize::new(100., 30.));
        let button = unsafe { NSButton::initWithFrame(NSButton::alloc(mtm), button_frame) };

        unsafe {
            button.setTitle(ns_string!("Open JP2"));
            button.setBezelStyle(NSBezelStyle::Rounded);

            let selector = sel!(openFile:);
            button.setAction(Some(selector));

            // Convert self to AnyObject for target
            let target: Option<&AnyObject> = Some(self.as_ref());
            button.setTarget(target);

            let content_view = window.contentView().unwrap();
            content_view.addSubview(&button);
        }
    }

    fn create_placeholder_image(&self, width: usize, height: usize) -> Option<Retained<NSImage>> {
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

            for y in 0..height {
                for x in 0..width {
                    let buffer_index = (y * bytes_per_row + x * 4) as isize;

                    // Create a blue to white gradient
                    let r = ((x as f64) / (width as f64) * 255.0) as u8;
                    let g = ((y as f64) / (height as f64) * 255.0) as u8;
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
