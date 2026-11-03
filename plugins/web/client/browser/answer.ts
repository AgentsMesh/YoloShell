export type AnswerResult = { text: string; ok: boolean; clear: boolean };

/** Map the Hub's CommandReceipt outcome to the answer card's next state. */
export function answerResult(outcome: unknown): AnswerResult {
  const value = (outcome ?? {}) as { state?: unknown; code?: unknown };
  switch (value.state) {
    case "delivered":
    case "confirmed":
      return { text: "Answer delivered", ok: true, clear: true };
    case "accepted":
      return { text: "Answer accepted; awaiting result", ok: true, clear: true };
    case "unknown":
      return { text: "Answer outcome unknown; refresh the Agent", ok: false, clear: false };
    case "failed":
      return {
        text:
          value.code === "preconditions_changed"
            ? "Answer rejected; refresh the Agent"
            : "Answer not delivered",
        ok: false,
        clear: false,
      };
    default:
      return { text: "Answer outcome unknown; refresh the Agent", ok: false, clear: false };
  }
}

export function outcomeLabel(value: unknown): string {
  switch (value) {
    case "delivered":
      return "Prompt delivered";
    case "turn_ended":
      return "Turn ended";
    case "blocked":
      return "Agent is waiting for approval";
    case "timed_out":
      return "Prompt sent; the turn is still running";
    case "exited":
      return "Agent exited";
    case "rejected":
      return "Prompt rejected";
    case "unknown":
      return "Delivery unconfirmed; check the conversation before resending";
    default:
      return "Prompt sent";
  }
}
