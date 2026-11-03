export class HubError extends Error {
  constructor(
    readonly code: number,
    message: string,
    readonly data: unknown,
  ) {
    super(message);
  }
}
