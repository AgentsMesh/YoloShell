export type ControlState = "observing" | "acquiring" | "controlled" | "lost" | "released";

export type ControlPresentation = {
  status: string;
  active: boolean;
  showTake: boolean;
};

export function controlPresentation(input: {
  state: ControlState;
  live: boolean;
  seeding: boolean;
  wants: boolean;
  leaseKnown: boolean;
  otherController?: boolean;
  viewers?: number;
}): ControlPresentation {
  const controlled = input.state === "controlled";
  const showTake =
    !controlled && !input.wants && input.live && !input.seeding && input.leaseKnown;
  if (input.seeding) return { status: "Loading screen…", active: false, showTake };
  if (!input.live) return { status: "Connecting terminal…", active: false, showTake };
  if (controlled) {
    const viewers = input.viewers ?? 1;
    return {
      status: viewers > 1 ? `Typing enabled · ${viewers} viewers` : "Typing enabled",
      active: true,
      showTake,
    };
  }
  if (input.wants || input.state === "acquiring") {
    return { status: "Taking control…", active: false, showTake };
  }
  return {
    status: input.otherController
      ? "Read-only · another tab controls"
      : "Read-only · type to take control",
    active: false,
    showTake,
  };
}
