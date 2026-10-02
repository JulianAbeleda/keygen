// AppKit owns composition and candidate selection; the host receives only
// bounded UTF-8 events. All entry points run on the window's main thread.
#import <AppKit/AppKit.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

@interface KGTextInputView : NSView <NSTextInputClient>
@property(nonatomic, weak) NSWindow *owner;
@property(nonatomic, weak) NSResponder *previous;
@property(nonatomic) BOOL enabled;
@property(nonatomic) NSRect caret;
@property(nonatomic, copy) NSString *marked;
@property(nonatomic) NSRange selection;
@property(nonatomic, strong) NSMutableArray<NSDictionary *> *events;
// What VoiceOver reads (the host's TextAccessibility); absent until set.
@property(nonatomic) BOOL axPresent;
@property(nonatomic, copy) NSString *axLabel;
@property(nonatomic, copy) NSString *axValue;
@property(nonatomic) NSRange axSelection;
@property(nonatomic) NSInteger axLine;
@property(nonatomic) NSRect axFrame;
- (NSRect)screenRect:(NSRect)rect;
- (void)emit:(NSUInteger)kind text:(NSString *)text;
- (void)cancelComposition;
@end

@implementation KGTextInputView
- (BOOL)acceptsFirstResponder { return YES; }
- (BOOL)isOpaque { return NO; }
- (NSView *)hitTest:(NSPoint)point { (void)point; return nil; }
- (void)emit:(NSUInteger)kind text:(NSString *)text {
    NSData *data = [text dataUsingEncoding:NSUTF8StringEncoding];
    // Refuse oversized input rather than splitting UTF-8 or silently dropping
    // the beginning of a committed string. At most 128 events await a frame.
    if (data.length > 4096 || self.events.count >= 128) return;
    [self.events addObject:@{@"kind": @(kind), @"text": data ?: [NSData data]}];
}
- (void)cancelComposition {
    if (self.marked.length) {
        self.marked = @"";
        [self.inputContext discardMarkedText];
        [self emit:3 text:@""];
    }
}
- (BOOL)resignFirstResponder {
    [self cancelComposition];
    return [super resignFirstResponder];
}
- (void)keyDown:(NSEvent *)event {
    if (!self.enabled || (event.modifierFlags & NSEventModifierFlagCommand)) {
        [self cancelComposition];
        [self.owner keyDown:event];
        return;
    }
    BOOL composing = self.hasMarkedText;
    [self.inputContext handleEvent:event];
    // Enter that accepts an IME candidate is not an application Return.
    // Ordinary keys still reach minifb's key-state callback; its duplicate
    // character callback is filtered by the Rust host while enabled.
    if (!composing && !self.hasMarkedText) [self.owner keyDown:event];
}
- (void)keyUp:(NSEvent *)event { [self.owner keyUp:event]; }
- (void)flagsChanged:(NSEvent *)event { [self.owner flagsChanged:event]; }
- (BOOL)hasMarkedText { return self.marked.length != 0; }
- (NSRange)markedRange {
    return self.hasMarkedText ? NSMakeRange(0, self.marked.length) : NSMakeRange(NSNotFound, 0);
}
- (NSRange)selectedRange { return self.selection; }
- (NSArray<NSAttributedStringKey> *)validAttributesForMarkedText { return @[]; }
- (void)setMarkedText:(id)value selectedRange:(NSRange)selected replacementRange:(NSRange)replacement {
    (void)replacement;
    NSString *text = [value isKindOfClass:[NSAttributedString class]] ? [value string] : value;
    if (![text isKindOfClass:[NSString class]] || [text lengthOfBytesUsingEncoding:NSUTF8StringEncoding] > 4096) return;
    self.marked = text;
    NSUInteger start = MIN(selected.location, text.length);
    self.selection = NSMakeRange(start, MIN(selected.length, text.length - start));
    [self emit:1 text:text];
}
- (void)unmarkText {
    if (self.marked.length) [self emit:2 text:self.marked];
    self.marked = @"";
    self.selection = NSMakeRange(0, 0);
    [self emit:3 text:@""];
}
- (void)insertText:(id)value replacementRange:(NSRange)replacement {
    (void)replacement;
    NSString *text = [value isKindOfClass:[NSAttributedString class]] ? [value string] : value;
    if (![text isKindOfClass:[NSString class]]) return;
    self.marked = @"";
    self.selection = NSMakeRange(0, 0);
    [self emit:2 text:text];
}
- (void)doCommandBySelector:(SEL)selector {
    if (selector == @selector(cancelOperation:)) [self cancelComposition];
    // Unconsumed command keys are forwarded once by keyDown, never here.
}
- (NSAttributedString *)attributedSubstringForProposedRange:(NSRange)range actualRange:(NSRangePointer)actual {
    if (range.location > self.marked.length) {
        if (actual) *actual = NSMakeRange(NSNotFound, 0);
        return nil;
    }
    range.length = MIN(range.length, self.marked.length - range.location);
    if (actual) *actual = range;
    return [[NSAttributedString alloc] initWithString:[self.marked substringWithRange:range]];
}
- (NSUInteger)characterIndexForPoint:(NSPoint)point { (void)point; return NSNotFound; }
- (NSRect)firstRectForCharacterRange:(NSRange)range actualRange:(NSRangePointer)actual {
    if (actual) *actual = NSMakeRange(MIN(range.location, self.marked.length), 0);
    return [self screenRect:self.caret];
}
// A content-local, top-left rectangle in screen coordinates.
- (NSRect)screenRect:(NSRect)rect {
    NSView *content = self.owner.contentView;
    rect.origin.y = content.bounds.size.height - rect.origin.y - rect.size.height;
    return [self.owner convertRectToScreen:[content convertRect:rect toView:nil]];
}
// The edited text as one AXTextArea: the app draws its own text, so this view
// stands in for it with the value, selection and insertion line it was given.
- (BOOL)isAccessibilityElement { return self.axPresent; }
- (NSAccessibilityRole)accessibilityRole { return NSAccessibilityTextAreaRole; }
- (NSString *)accessibilityLabel { return self.axLabel; }
- (id)accessibilityValue { return self.axValue ?: @""; }
- (NSInteger)accessibilityNumberOfCharacters { return (NSInteger)self.axValue.length; }
- (NSRange)accessibilitySelectedTextRange { return self.axSelection; }
- (NSString *)accessibilitySelectedText {
    return NSMaxRange(self.axSelection) <= self.axValue.length ? [self.axValue substringWithRange:self.axSelection] : @"";
}
- (NSInteger)accessibilityInsertionPointLineNumber { return self.axLine; }
- (NSRect)accessibilityFrame { return [self screenRect:self.axFrame]; }
- (BOOL)isAccessibilityFocused { return self.owner.firstResponder == self; }
@end

// The integer handle is validated by identity against AppKit's live window
// list before it is ever messaged. No untrusted address is dereferenced.
void *kg_text_input_attach(uintptr_t identity) {
    if (![NSThread isMainThread]) return NULL;
    NSWindow *owner = nil;
    for (NSWindow *window in NSApp.windows) {
        if ((uintptr_t)(__bridge void *)window == identity) { owner = window; break; }
    }
    if (!owner || !owner.contentView) return NULL;
    KGTextInputView *view = [[KGTextInputView alloc] initWithFrame:NSZeroRect];
    view.owner = owner;
    view.previous = owner.firstResponder;
    view.marked = @"";
    view.events = [NSMutableArray array];
    [owner.contentView addSubview:view];
    return (__bridge_retained void *)view;
}
void kg_text_input_set(void *handle, bool enabled, double x, double y, double width, double height) {
    KGTextInputView *view = (__bridge KGTextInputView *)handle;
    if (!enabled) [view cancelComposition];
    view.enabled = enabled;
    NSRect caret = NSMakeRect(x, y, width, height);
    if (!NSEqualRects(view.caret, caret)) {
        view.caret = caret;
        [view.inputContext invalidateCharacterCoordinates];
    }
    if (enabled && view.owner.firstResponder != view) [view.owner makeFirstResponder:view];
    if (!enabled && view.owner.firstResponder == view) [view.owner makeFirstResponder:view.previous];
}
// Ranges are UTF-16 code units, as AppKit counts them. Posts the change
// notifications VoiceOver listens for, only for what changed.
void kg_text_input_accessibility(void *handle, bool present, const uint8_t *label, size_t label_len,
                                 const uint8_t *value, size_t value_len, size_t start, size_t length,
                                 size_t line, double x, double y, double width, double height) {
    KGTextInputView *view = (__bridge KGTextInputView *)handle;
    NSString *text = [[NSString alloc] initWithBytes:value length:value_len encoding:NSUTF8StringEncoding] ?: @"";
    BOOL value_changed = ![text isEqualToString:view.axValue ?: @""];
    BOOL selection_changed = !NSEqualRanges(view.axSelection, NSMakeRange(start, length));
    view.axPresent = present;
    view.axLabel = [[NSString alloc] initWithBytes:label length:label_len encoding:NSUTF8StringEncoding];
    view.axValue = text;
    view.axSelection = NSMakeRange(MIN(start, text.length), MIN(length, text.length - MIN(start, text.length)));
    view.axLine = (NSInteger)line;
    view.axFrame = NSMakeRect(x, y, width, height);
    if (present && value_changed) NSAccessibilityPostNotification(view, NSAccessibilityValueChangedNotification);
    if (present && selection_changed)
        NSAccessibilityPostNotification(view, NSAccessibilitySelectedTextChangedNotification);
}
NSUInteger kg_text_input_poll(void *handle, uint8_t *buffer, NSUInteger capacity, NSUInteger *length) {
    KGTextInputView *view = (__bridge KGTextInputView *)handle;
    NSDictionary *event = view.events.firstObject;
    if (!event) return 0;
    NSData *text = event[@"text"];
    if (text.length > capacity) return 0;
    memcpy(buffer, text.bytes, text.length);
    *length = text.length;
    NSUInteger kind = [event[@"kind"] unsignedIntegerValue];
    [view.events removeObjectAtIndex:0];
    return kind;
}
void kg_text_input_detach(void *handle) {
    KGTextInputView *view = (__bridge_transfer KGTextInputView *)handle;
    [view cancelComposition];
    if (view.owner.firstResponder == view) [view.owner makeFirstResponder:view.previous];
    [view removeFromSuperview];
}

// The pointer shape, set unconditionally. minifb sets a cursor only when its
// style changes, and AppKit resets the cursor to the arrow on activation and
// at window edges, so a shape the app still wants is never shown again until
// the app changes its mind. The host calls this to reassert the current shape.
// The person's double-click time (System Settings > Accessibility), seconds.
double kg_double_click_interval(void) {
    return [NSEvent doubleClickInterval];
}

void kg_cursor_set(int32_t shape) {
    [(shape == 1 ? [NSCursor IBeamCursor] : [NSCursor arrowCursor]) set];
}
