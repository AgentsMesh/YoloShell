@0xb469e3ad5e8a1be2;

using Common = import "common.capnp";

interface Echo {
  echo @0 (note :Common.Note) -> (note :Common.Note);
}
