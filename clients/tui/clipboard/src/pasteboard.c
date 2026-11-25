#include <ApplicationServices/ApplicationServices.h>
#include <CoreFoundation/CoreFoundation.h>
#include <ImageIO/ImageIO.h>
#include <stdint.h>

typedef void (*ClipboardBytes)(void *context, const void *bytes, size_t length);

static void copy_data(CFDataRef value, ClipboardBytes copy, void *context) {
    copy(context, CFDataGetBytePtr(value), (size_t)CFDataGetLength(value));
}

static CFDataRef copy_png(CFDataRef input) {
    CGImageSourceRef source = CGImageSourceCreateWithData(input, NULL);
    if (!source) return NULL;
    CFMutableDataRef result = CFDataCreateMutable(NULL, 0);
    CGImageDestinationRef destination = result
        ? CGImageDestinationCreateWithData(result, CFSTR("public.png"), 1, NULL) : NULL;
    Boolean ok = false;
    if (destination && CGImageSourceGetCount(source)) {
        CGImageDestinationAddImageFromSource(destination, source, 0, NULL);
        ok = CGImageDestinationFinalize(destination);
    }
    if (destination) CFRelease(destination);
    CFRelease(source);
    if (ok) return result;
    if (result) CFRelease(result);
    return NULL;
}

static Boolean copy_file_urls(PasteboardRef board, ItemCount count, ClipboardBytes copy, void *context) {
    CFMutableDataRef urls = CFDataCreateMutable(NULL, 0);
    if (!urls) return false;
    for (ItemCount i = 1; i <= count; i++) {
        PasteboardItemID id;
        CFDataRef url;
        if (PasteboardGetItemIdentifier(board, i, &id) != noErr ||
            PasteboardCopyItemFlavorData(board, id, CFSTR("public.file-url"), &url) != noErr) continue;
        if (CFDataGetLength(url)) {
            if (CFDataGetLength(urls)) CFDataAppendBytes(urls, (const UInt8 *)"\n", 1);
            CFDataAppendBytes(urls, CFDataGetBytePtr(url), CFDataGetLength(url));
        }
        CFRelease(url);
    }
    Boolean found = CFDataGetLength(urls) != 0;
    if (found) copy_data(urls, copy, context);
    CFRelease(urls);
    return found;
}

static uint8_t read_items(PasteboardRef board, ItemCount count, ClipboardBytes copy, void *context) {
    if (copy_file_urls(board, count, copy, context)) return 'F';
    CFStringRef types[] = { CFSTR("public.png"), CFSTR("public.jpeg"), CFSTR("public.tiff") };
    for (ItemCount i = 1; i <= count; i++) {
        PasteboardItemID id;
        if (PasteboardGetItemIdentifier(board, i, &id) != noErr) continue;
        for (size_t j = 0; j < sizeof(types) / sizeof(types[0]); j++) {
            CFDataRef data;
            if (PasteboardCopyItemFlavorData(board, id, types[j], &data) != noErr) continue;
            if (CFEqual(types[j], CFSTR("public.tiff"))) {
                CFDataRef png = copy_png(data);
                CFRelease(data);
                if (!png) continue;
                data = png;
            }
            Boolean found = CFDataGetLength(data) != 0;
            if (found) copy_data(data, copy, context);
            CFRelease(data);
            if (found) return CFEqual(types[j], CFSTR("public.jpeg")) ? 'J' : 'P';
        }
    }
    return 0;
}

// A NULL name selects the user's clipboard. Named pasteboards support isolated validation.
uint8_t yolo_read_pasteboard(const char *name, ClipboardBytes copy, void *context) {
    CFStringRef label = name ? CFStringCreateWithCString(NULL, name, kCFStringEncodingUTF8) : NULL;
    if (name && !label) return 0;
    PasteboardRef board;
    OSStatus status = PasteboardCreate(label ? label : kPasteboardClipboard, &board);
    if (label) CFRelease(label);
    if (status != noErr) return 0;
    PasteboardSynchronize(board);
    ItemCount count = 0;
    uint8_t tag = PasteboardGetItemCount(board, &count) == noErr ? read_items(board, count, copy, context) : 0;
    CFRelease(board);
    return tag;
}
