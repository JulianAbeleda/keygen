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
    NSView *content = self.owner.contentView;
    NSRect rect = self.caret;
    rect.origin.y = content.bounds.size.height - rect.origin.y - rect.size.height;
    return [self.owner convertRectToScreen:[content convertRect:rect toView:nil]];
}
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
