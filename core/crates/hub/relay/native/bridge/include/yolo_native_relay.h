#ifndef YOLO_NATIVE_RELAY_H
#define YOLO_NATIVE_RELAY_H
#include <stddef.h>
#include <stdint.h>

typedef struct { uint64_t high; uint64_t low; } YoloRelayIdentity;
typedef struct { const uint8_t *data; size_t length; } YoloRelayText;
typedef struct { uint8_t *data; size_t length; } YoloRelayBuffer;
typedef struct {
  YoloRelayIdentity launch, hub, terminal, runtime, request, agent, execution, stream;
  uint64_t sequence;
  uint32_t action;
  YoloRelayText prompt;
} YoloRelayRequest;
typedef struct {
  uint32_t kind, state, failure, reason;
} YoloRelayOutcome;
typedef struct {
  uint32_t kind;
  YoloRelayRequest request;
  YoloRelayOutcome outcome;
  uint32_t status;
  YoloRelayBuffer prompt, holder;
} YoloRelayDecoded;

enum { YOLO_RELAY_OK = 0, YOLO_RELAY_INVALID = 1, YOLO_RELAY_PANIC = 2 };
enum { YOLO_RELAY_REQUEST = 1, YOLO_RELAY_RESPONSE = 2, YOLO_RELAY_STATUS = 3 };
enum { YOLO_RELAY_INTERRUPT = 0, YOLO_RELAY_SUBMIT = 1 };
enum { YOLO_RELAY_OUTCOME_UNKNOWN = 0, YOLO_RELAY_NOT_SENT = 1, YOLO_RELAY_RECEIPT = 2 };
enum { YOLO_RELAY_ACCEPTED = 0, YOLO_RELAY_DELIVERED = 1, YOLO_RELAY_CONFIRMED = 2,
       YOLO_RELAY_RECEIPT_UNKNOWN = 3, YOLO_RELAY_FAILED = 4 };
enum { YOLO_RELAY_PERMISSION_DENIED = 0, YOLO_RELAY_PRECONDITIONS_CHANGED = 1,
       YOLO_RELAY_UNSUPPORTED = 2, YOLO_RELAY_EXPIRED = 3, YOLO_RELAY_ADAPTER_REJECTED = 4 };
enum { YOLO_RELAY_NO_WRITER = 0, YOLO_RELAY_NOT_SENT_EXPIRED = 1,
       YOLO_RELAY_TARGET_CHANGED = 2, YOLO_RELAY_UNAVAILABLE = 3 };
enum { YOLO_RELAY_CONTROL_ACQUIRED = 0, YOLO_RELAY_READ_ONLY = 1, YOLO_RELAY_CONTROL_LOST = 2,
       YOLO_RELAY_HOST_STARTED = 3, YOLO_RELAY_SESSION_RESUMED = 4,
       YOLO_RELAY_SESSION_ENDING = 5, YOLO_RELAY_SESSION_EXITED = 6 };

// Input storage is borrowed only for the call. Outputs own their buffers until released.
// Input/output pointers must refer to valid, non-overlapping storage.
int32_t yolo_native_relay_encode_request(const YoloRelayRequest *, YoloRelayBuffer *);
int32_t yolo_native_relay_encode_response(const YoloRelayRequest *, const YoloRelayOutcome *, YoloRelayBuffer *);
int32_t yolo_native_relay_encode_status(uint32_t kind, YoloRelayText holder, YoloRelayBuffer *);
int32_t yolo_native_relay_decode(const uint8_t *, size_t, YoloRelayDecoded *);
void yolo_native_relay_release_buffer(YoloRelayBuffer *);
void yolo_native_relay_release_decoded(YoloRelayDecoded *);
int32_t yolo_native_startup_encode_commit(uint32_t process_id, YoloRelayBuffer *);
size_t yolo_native_startup_maximum_frame_bytes(void);
int32_t yolo_native_startup_validate_committed(const uint8_t *, size_t, uint32_t process_id);
#endif
