@0xc4f8d6b3e1a29705;

struct Identity {
  high @0 :UInt64;
  low @1 :UInt64;
}

struct ControlRequest {
  launch @0 :Identity;
  hub @1 :Identity;
  terminal @2 :Identity;
  runtime @3 :Identity;
  request @4 :Identity;
  agent @5 :Identity;
  execution @6 :Identity;
  stream @7 :Identity;
  sequence @8 :UInt64;
  action :union {
    interrupt @9 :Void;
    prompt @10 :Text;
  }
}

enum Failure {
  permissionDenied @0;
  preconditionsChanged @1;
  unsupported @2;
  expired @3;
  adapterRejected @4;
}

enum NotSent {
  noWriter @0;
  expired @1;
  targetChanged @2;
  unavailable @3;
}

struct ControlResponse {
  request @0 :ControlRequest;
  outcome :union {
    accepted @1 :Void;
    delivered @2 :Void;
    confirmed @3 :Void;
    receiptUnknown @4 :Void;
    failed @5 :Failure;
    notSent @6 :NotSent;
    unknown @7 :Void;
  }
}

enum StatusKind {
  controlAcquired @0;
  readOnly @1;
  controlLost @2;
  hostStarted @3;
  sessionResumed @4;
  sessionEnding @5;
  sessionExited @6;
}

struct Status {
  kind @0 :StatusKind;
  holder :union {
    none @1 :Void;
    id @2 :Text;
  }
}

struct Frame {
  union {
    request @0 :ControlRequest;
    response @1 :ControlResponse;
    status @2 :Status;
  }
}
