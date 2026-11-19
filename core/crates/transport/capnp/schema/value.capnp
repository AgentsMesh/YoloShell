@0xe7fd8d528bd3d412;

# Structural values preserve the existing validated domain DTOs without embedding
# JSON or another wire format. Domain services own method and identity validation.
struct Value {
  union {
    null @0 :Void;
    boolean @1 :Bool;
    signed @2 :Int64;
    unsigned @3 :UInt64;
    float @4 :Float64;
    text @5 :Text;
    data @6 :Data;
    list @7 :List(Value);
    map @8 :List(Entry);
  }
}

struct Entry {
  key @0 :Value;
  value @1 :Value;
}

struct Document {
  version @0 :UInt16;
  value @1 :Value;
}
