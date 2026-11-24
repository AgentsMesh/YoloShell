@0xafeb2b27e9089f27;

struct Note {
  sequence @0 :UInt64;
  text @1 :Text;
  union {
    pending @2 :Void;
    complete @3 :Bool;
  }
}
