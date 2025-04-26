mod appdelegate;

use appdelegate::setup_delegate;

use cocoa::appkit::{NSApp, NSApplication, NSBackingStoreType, NSButton, NSView, NSWindow};
use cocoa::base::{id, nil, selector, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};

fn main() {
    unsafe {
        // Create autorelease pool
        let _pool = NSAutoreleasePool::new(nil);

        // Create the application
        let app = NSApp();
        app.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicyRegular);
        // Setup our delegate
        let delegate = setup_delegate();
        app.setDelegate_(delegate);

        // Create a window
        let window_frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(800., 600.));
        let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            window_frame,
            cocoa::appkit::NSWindowStyleMask::NSTitledWindowMask
                | cocoa::appkit::NSWindowStyleMask::NSClosableWindowMask
                | cocoa::appkit::NSWindowStyleMask::NSResizableWindowMask,
            NSBackingStoreType::NSBackingStoreBuffered,
            NO,
        );

        // Set window properties
        let title = cocoa::foundation::NSString::alloc(nil).init_str("JP2 Viewer");
        NSWindow::setTitle_(window, title);

        window.makeKeyAndOrderFront_(nil);
        window.center();

        // Create a button
        let button_frame = NSRect::new(NSPoint::new(350., 30.), NSSize::new(100., 30.));
        let button = NSButton::initWithFrame_(NSButton::alloc(nil), button_frame);
        let button_title = cocoa::foundation::NSString::alloc(nil).init_str("Open JP2");
        NSButton::setTitle_(button, button_title);
        button.setBezelStyle_(cocoa::appkit::NSBezelStyle::NSRoundedBezelStyle);
        button.setAction_(selector("openFile:"));

        // Add button to window's content view
        let content_view: id = window.contentView();
        content_view.addSubview_(button);

        // Create application delegate to handle actions
        // (We'll add this later)

        // Activate the application
        let process_info: id = msg_send![class!(NSProcessInfo), processInfo];
        let process_name: id = msg_send![process_info, processName];
        app.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicyRegular);
        app.activateIgnoringOtherApps_(YES);

        // Run the application
        app.run();
    }
}
