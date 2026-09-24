// Optional native AppKit contract check; never run by the headless test suite.
// From the repository root:
// clang -fobjc-arc -framework AppKit crates/macos/tests/text_input.m -o /tmp/keygen-text-input-test
// /tmp/keygen-text-input-test
// This exercises the platform protocol, not a human-operated input source.
#import "../src/text_input.m"
#include <assert.h>
#include <stdio.h>

static void expect(void *handle, NSUInteger kind, NSString *value) {
    uint8_t bytes[4096];
    NSUInteger length = 0;
    assert(kg_text_input_poll(handle, bytes, sizeof(bytes), &length) == kind);
    NSString *actual = [[NSString alloc] initWithBytes:bytes length:length encoding:NSUTF8StringEncoding];
    assert([actual isEqualToString:value]);
}

int main(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        assert(kg_text_input_attach(1) == NULL);
        NSWindow *window = [[NSWindow alloc]
            initWithContentRect:NSMakeRect(100, 100, 640, 480)
            styleMask:NSWindowStyleMaskTitled backing:NSBackingStoreBuffered defer:NO];
        window.releasedWhenClosed = NO;
        void *handle = kg_text_input_attach((uintptr_t)(__bridge void *)window);
        assert(handle != NULL);
        KGTextInputView *view = (__bridge KGTextInputView *)handle;
        kg_text_input_set(handle, true, 40, 100, 1, 20);
        assert(window.firstResponder == view);
        [view setMarkedText:@"にほん🦀" selectedRange:NSMakeRange(3, 2)
            replacementRange:NSMakeRange(NSNotFound, 0)];
        assert(view.hasMarkedText);
        assert(view.selectedRange.location == 3 && view.selectedRange.length == 2);
        expect(handle, 1, @"にほん🦀");
        NSRect rect = [view firstRectForCharacterRange:NSMakeRange(0, 0) actualRange:NULL];
        assert(rect.size.width == 1 && rect.size.height == 20);
        [view insertText:[[NSAttributedString alloc] initWithString:@"日本🦀"]
            replacementRange:NSMakeRange(NSNotFound, 0)];
        assert(!view.hasMarkedText);
        expect(handle, 2, @"日本🦀");
        [view setMarkedText:@"你好" selectedRange:NSMakeRange(99, 99)
            replacementRange:NSMakeRange(NSNotFound, 0)];
        assert(view.selectedRange.location == 2 && view.selectedRange.length == 0);
        expect(handle, 1, @"你好");
        [view unmarkText];
        expect(handle, 2, @"你好");
        expect(handle, 3, @"");
        [view setMarkedText:@"cancel" selectedRange:NSMakeRange(0, 0)
            replacementRange:NSMakeRange(NSNotFound, 0)];
        expect(handle, 1, @"cancel");
        kg_text_input_set(handle, false, 0, 0, 1, 1);
        assert(!view.hasMarkedText && window.firstResponder != view);
        expect(handle, 3, @"");
        // The Rust guard may outlive its closed window without a dangling owner.
        [window close];
        kg_text_input_detach(handle);
        puts("AppKit text-input protocol contracts passed (not live IME qualification)");
    }
    return 0;
}
