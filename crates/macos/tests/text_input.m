// Optional native AppKit contract check; never run by the headless test suite.
// From the repository root:
// clang -fobjc-arc -framework AppKit crates/macos/tests/text_input.m -o /tmp/keygen-text-input-test
// /tmp/keygen-text-input-test
// This exercises the platform protocol, not a human-operated input source.
// Opt-in real system IMEs with injected keyboard events (sources must be enabled):
// /tmp/keygen-text-input-test com.apple.inputmethod.Kotoeri.RomajiTyping.Japanese nihon
// /tmp/keygen-text-input-test com.apple.inputmethod.SCIM.ITABC nihao
#import "../src/text_input.m"
#include <assert.h>
#include <stdio.h>

@interface KGTestWindow : NSWindow
@property(nonatomic) NSUInteger returns;
@end
@implementation KGTestWindow
- (void)keyDown:(NSEvent *)event {
    if (event.keyCode == 36) self.returns++;
}
@end

static void pump(void) {
    NSDate *until = [NSDate dateWithTimeIntervalSinceNow:0.15];
    while (until.timeIntervalSinceNow > 0) {
        NSEvent *event = [NSApp nextEventMatchingMask:NSEventMaskAny untilDate:until
            inMode:NSDefaultRunLoopMode dequeue:YES];
        if (event) [NSApp sendEvent:event];
    }
}

// Explicit opt-in live input-source check: enable the source in macOS first.
// Pass its exact ID and romaji/pinyin; U.S. selection is restored afterward.
static int live(NSString *source, NSString *spelling) {
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    [NSApp finishLaunching];
    KGTestWindow *window = [[KGTestWindow alloc] initWithContentRect:NSMakeRect(100, 100, 640, 480)
        styleMask:NSWindowStyleMaskTitled backing:NSBackingStoreBuffered defer:NO];
    window.releasedWhenClosed = NO;
    window.title = @"KeyGen native IME qualification";
    [window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
    void *handle = kg_text_input_attach((uintptr_t)(__bridge void *)window);
    assert(handle);
    KGTextInputView *view = (__bridge KGTextInputView *)handle;
    kg_text_input_set(handle, true, 60, 100, 1, 20);
    pump();
    [view.inputContext activate];
    view.inputContext.selectedKeyboardInputSource = source;
    pump();
    printf("context=%p selected=%s key_window=%d active=%d\n", view.inputContext,
        view.inputContext.selectedKeyboardInputSource.UTF8String, window.isKeyWindow, NSApp.isActive);
    NSDictionary *codes = @{@"n": @45, @"i": @34, @"h": @4, @"o": @31, @"a": @0, @" ": @49, @"\r": @36};
    NSString *sequence = [spelling stringByAppendingString:@" \r\r\r"];
    NSMutableString *committed = [NSMutableString string];
    BOOL sawPreedit = NO;
    NSUInteger expectedReturns = 0;
    for (NSUInteger at = 0; at < sequence.length; at++) {
        NSString *character = [sequence substringWithRange:NSMakeRange(at, 1)];
        if ([character isEqualToString:@"\r"] && !view.hasMarkedText) expectedReturns++;
        CGEventRef keyEvent = CGEventCreateKeyboardEvent(NULL, [codes[character] unsignedShortValue], true);
        [view keyDown:[NSEvent eventWithCGEvent:keyEvent]];
        CFRelease(keyEvent);
        pump();
        uint8_t bytes[4096]; NSUInteger length = 0, kind;
        while ((kind = kg_text_input_poll(handle, bytes, sizeof(bytes), &length))) {
            NSString *value = [[NSString alloc] initWithBytes:bytes length:length encoding:NSUTF8StringEncoding];
            printf("event=%lu text=%s\n", (unsigned long)kind, value.UTF8String);
            if (kind == 1) sawPreedit = YES;
            if (kind == 2) [committed appendString:value];
        }
    }
    BOOL japanese = [source containsString:@"Kotoeri"];
    BOOL passed = sawPreedit && [committed isEqualToString:japanese ? @"日本" : @"你好"]
        && window.returns == expectedReturns && window.returns > 0;
    printf("source=%s committed=%s raw_returns=%lu passed=%d\n", source.UTF8String,
        committed.UTF8String, (unsigned long)window.returns, passed);
    [view cancelComposition];
    view.inputContext.selectedKeyboardInputSource = @"com.apple.keylayout.US";
    kg_text_input_detach(handle);
    [window close];
    return passed ? 0 : 1;
}

static void expect(void *handle, NSUInteger kind, NSString *value) {
    uint8_t bytes[4096];
    NSUInteger length = 0;
    assert(kg_text_input_poll(handle, bytes, sizeof(bytes), &length) == kind);
    NSString *actual = [[NSString alloc] initWithBytes:bytes length:length encoding:NSUTF8StringEncoding];
    assert([actual isEqualToString:value]);
}

int main(int argc, const char **argv) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        if (argc == 3) return live(@(argv[1]), @(argv[2]));
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
