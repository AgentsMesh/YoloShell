import { hubEndpoint } from "./hub";
import { entryMachine, hubFor } from "./machineHandles";
import { record, uuid } from "./validate";

export type TerminalTarget = {
  address: { hub_id: string; terminal_id: string };
  runtime_epoch: string;
};

export function terminalTarget(value: unknown): TerminalTarget {
  const item = record(value);
  const machine = item.machine === undefined || item.machine === null
    ? entryMachine()
    : uuid(item.machine);
  return {
    address: {
      hub_id: hubFor(machine),
      terminal_id: uuid(item.terminal_id),
    },
    runtime_epoch: uuid(item.runtime_epoch),
  };
}
