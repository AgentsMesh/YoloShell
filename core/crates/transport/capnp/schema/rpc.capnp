@0xb433a1f5bc1f5043;
using V = import "value.capnp";

struct Result {
  value @0 :V.Value;
  events @1 :Events;
  receipt @2 :Receipt;
}

interface Receipt {
  confirm @0 ();
}

interface Events {
  next @0 () -> (event :V.Value);
  cancel @1 ();
}

interface Endpoint {
  call @0 (request :V.Value) -> (result :Result);
}

interface Peer {
  discover @0 (request :V.Value) -> (result :Result);
  execute @1 (request :V.Value) -> (result :Result);
}
