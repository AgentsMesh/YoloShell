#ifndef YOLO_NATIVE_VALUE_H
#define YOLO_NATIVE_VALUE_H
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

typedef struct YoloNativeValue YoloNativeValue;
typedef struct { uint8_t *data; size_t length; } YoloNativeBuffer;
enum { YOLO_NATIVE_NULL=0, YOLO_NATIVE_BOOLEAN=1, YOLO_NATIVE_SIGNED=2,
    YOLO_NATIVE_UNSIGNED=3, YOLO_NATIVE_FLOAT=4, YOLO_NATIVE_TEXT=5,
    YOLO_NATIVE_DATA=6, YOLO_NATIVE_LIST=7, YOLO_NATIVE_MAP=8, YOLO_NATIVE_INVALID=255 };
/* Status: 0 success, 1 invalid input, 2 caught panic. All pointers must be live.
 * Only release owned roots. Child getters borrow until their root is released.
 * Successful push/insert transfers child ownership; failure transfers nothing.
 * Encoded buffers belong to Rust and must use release_buffer, never free(). */
int32_t yolo_native_value_decode(const uint8_t*, size_t, YoloNativeValue**);
int32_t yolo_native_value_encode(const YoloNativeValue*, YoloNativeBuffer*);
void yolo_native_value_release(YoloNativeValue*);
void yolo_native_value_release_buffer(YoloNativeBuffer*);
uint32_t yolo_native_value_tag(const YoloNativeValue*);
bool yolo_native_value_get_bool(const YoloNativeValue*);
int64_t yolo_native_value_get_i64(const YoloNativeValue*);
uint64_t yolo_native_value_get_u64(const YoloNativeValue*);
double yolo_native_value_get_f64(const YoloNativeValue*);
const uint8_t* yolo_native_value_get_bytes(const YoloNativeValue*, size_t*);
size_t yolo_native_value_get_count(const YoloNativeValue*);
const YoloNativeValue* yolo_native_value_get_child(const YoloNativeValue*, size_t);
const YoloNativeValue* yolo_native_value_get_key(const YoloNativeValue*, size_t);
const YoloNativeValue* yolo_native_value_get_value(const YoloNativeValue*, size_t);
YoloNativeValue* yolo_native_value_null(void);
YoloNativeValue* yolo_native_value_boolean(bool);
YoloNativeValue* yolo_native_value_signed(int64_t);
YoloNativeValue* yolo_native_value_unsigned(uint64_t);
YoloNativeValue* yolo_native_value_float(double);
YoloNativeValue* yolo_native_value_text(const uint8_t*, size_t);
YoloNativeValue* yolo_native_value_data(const uint8_t*, size_t);
YoloNativeValue* yolo_native_value_list(void);
YoloNativeValue* yolo_native_value_map(void);
int32_t yolo_native_value_push(YoloNativeValue*, YoloNativeValue*);
int32_t yolo_native_value_insert(YoloNativeValue*, YoloNativeValue*, YoloNativeValue*);
#endif
